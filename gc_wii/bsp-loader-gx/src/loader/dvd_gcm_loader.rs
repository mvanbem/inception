use core::mem::size_of;

use crate::loader::Loader;

use aligned::{Aligned, A32};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bytemuck::{from_bytes, Pod, Zeroable};
use inception_render_common::map_data::MapData;
use ogc_sys::GlobalAlign32;

pub struct DvdGcmLoader {
    table_data: Vec<u8, GlobalAlign32>,
    string_table_start: usize,
}

impl DvdGcmLoader {
    fn string(&self, offset: usize) -> &str {
        let start = self.string_table_start + offset;
        let end = start
            + self.table_data[start..]
                .iter()
                .copied()
                .position(|b| b == 0)
                .unwrap();
        core::str::from_utf8(&self.table_data[start..end]).unwrap()
    }

    fn read_file(&self, path: &str) -> Vec<u8, GlobalAlign32> {
        let orig_path = path;
        let mut path = path;

        let mut dir_offset = size_of::<FileTableEntry>();
        let mut end_offset = self.string_table_start;
        while dir_offset < end_offset {
            let entry: &FileTableEntry =
                from_bytes(&self.table_data[dir_offset..dir_offset + size_of::<FileTableEntry>()]);
            let name = self.string(entry.name_offset());

            if entry.is_file() {
                if name.eq_ignore_ascii_case(path) {
                    // Found the file. Read it.
                    let file_offset = entry.data_or_parent_index;
                    let file_size = entry.file_length_or_next_index;
                    let mut data = Vec::with_capacity_in((file_size + 31) & !31, GlobalAlign32);
                    gamecube_dvd_driver::read_maybe_uninit(file_offset, data.spare_capacity_mut())
                        .unwrap();
                    unsafe { data.set_len(file_size) }
                    return data;
                } else {
                    // Fall through and try the next entry.

                    dir_offset += size_of::<FileTableEntry>();
                }
            } else {
                let mut iter = path.splitn(2, '/');
                let dir_name = iter.next().unwrap();

                let next_offset = size_of::<FileTableEntry>() * entry.file_length_or_next_index;

                if name.eq_ignore_ascii_case(dir_name) {
                    // Found the next directory. Pop a component off the front of the path and
                    // update the search bounds.
                    let sub_path = iter.next().unwrap();
                    path = sub_path;
                    end_offset = next_offset;

                    dir_offset += size_of::<FileTableEntry>();
                } else {
                    // No match. Skip this directory.
                    dir_offset = next_offset;
                }
            }
        }

        panic!("File not found: {:?}", orig_path);
    }
}

impl Loader for DvdGcmLoader {
    type Params = ();
    type Data = Vec<u8, GlobalAlign32>;

    fn new(_: ()) -> Self {
        // Check for the expected disc.
        loop {
            unsafe {
                libc::printf(b"Resetting the disc drive...\n\0".as_ptr());
                gamecube_dvd_driver::reset();
                match gamecube_dvd_driver::read_disc_id() {
                    Ok(disc_id) => {
                        if &disc_id[..8] == b"GGMEMV\x00\x00" {
                            break;
                        }
                    }
                    Err(_) => (),
                }

                libc::printf(b"Unrecognized disc. Open the disc cover.\n\0".as_ptr());
                gamecube_dvd_driver::wait_for_cover(true);

                libc::printf(b"Insert the Inception disc and close the cover.\n\0".as_ptr());
                gamecube_dvd_driver::wait_for_cover(false);
            }
        }

        let mut metadata: Aligned<A32, _> = Aligned(DiscHeader0x420::zeroed());
        gamecube_dvd_driver::read(0x420, bytemuck::bytes_of_mut(&mut *metadata)).unwrap();

        let mut table_data = Vec::with_capacity_in((metadata.fst_size + 31) & !31, GlobalAlign32);
        gamecube_dvd_driver::read_maybe_uninit(
            metadata.fst_offset,
            table_data.spare_capacity_mut(),
        )
        .unwrap();
        unsafe { table_data.set_len(metadata.fst_size) }

        let root_entry: &FileTableEntry = from_bytes(&table_data[..size_of::<FileTableEntry>()]);
        let string_table_start = size_of::<FileTableEntry>() * root_entry.file_length_or_next_index;

        Self {
            table_data,
            string_table_start,
        }
    }

    fn maps(&mut self) -> Vec<String> {
        let mut maps = Vec::new();
        for bytes in self.read_file("maps.txt").split(|&b| b == b'\n') {
            if !bytes.is_empty() {
                maps.push(core::str::from_utf8(bytes).unwrap().to_string());
            }
        }
        maps
    }

    fn load_map(&mut self, map: &str) -> MapData<Self::Data> {
        let data = self.read_file(&format!("maps/{}.dat", map));
        unsafe { MapData::new(data) }
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct DiscHeader0x420 {
    _unused1: usize,
    fst_offset: usize,
    fst_size: usize,
    _unused2: [usize; 5],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct FileTableEntry {
    flags_and_name_offset: usize,
    data_or_parent_index: usize,
    file_length_or_next_index: usize,
}

impl FileTableEntry {
    fn is_file(&self) -> bool {
        self.flags_and_name_offset & 0xff000000 == 0
    }

    fn name_offset(&self) -> usize {
        self.flags_and_name_offset & 0x00ffffff
    }
}
