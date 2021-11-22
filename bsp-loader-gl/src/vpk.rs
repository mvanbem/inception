use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::mem::size_of;
use std::path::{Path, PathBuf};

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use memmap::Mmap;
use try_insert_ext::EntryInsertExt;

use crate::file::canonical_path::CanonicalPathBuf;
use crate::file::FileLoader;
use crate::transmute_utils::extract_at;
use crate::vpk::path::VpkPath;

pub mod path;

fn read_null_terminated_string<'a>(data: &mut &'a [u8]) -> &'a str {
    let (str_data, tail) = data.split_at(
        data.iter()
            .enumerate()
            .find(|&(_index, &b)| b == 0)
            .unwrap()
            .0
            + 1,
    );
    *data = tail;
    CStr::from_bytes_with_nul(str_data)
        .unwrap()
        .to_str()
        .unwrap()
}

pub struct Vpk {
    base_name: String,
    index_data: Mmap,
    entries_by_extension_parent_file_stem: HashMap<
        CanonicalPathBuf,
        HashMap<CanonicalPathBuf, HashMap<CanonicalPathBuf, DirectoryEntry>>,
    >,
    locked: RefCell<VpkLocked>,
}

struct VpkLocked {
    path: PathBuf,
    archives: HashMap<u16, Mmap>,
}

impl Vpk {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let mut path = base_path.as_ref().to_path_buf();
        let base_name = path.file_name().unwrap().to_str().unwrap().to_string();
        path.pop();

        let index_file_name = format!("{}_dir.vpk", base_name);
        path.push(index_file_name);
        let index_file = File::open(&path)?;
        path.pop();
        let index_data = unsafe { Mmap::map(&index_file) }?;

        // SAFETY: All bit patterns are valid for HeaderV1.
        let v1_header: &HeaderV1 = unsafe { extract_at(&*index_data, 0) };
        assert_eq!(v1_header.version, 2);

        // SAFETY: All bit patterns are valid for HeaderV2.
        let v2_header: &HeaderV2 = unsafe { extract_at(&*index_data, 0) };
        let mut tree_data = &(&index_data[size_of::<HeaderV2>()..])[..v2_header.tree_size as usize];
        let mut entries_by_extension_parent_file_stem: HashMap<
            CanonicalPathBuf,
            HashMap<CanonicalPathBuf, HashMap<CanonicalPathBuf, DirectoryEntry>>,
        > = HashMap::new();
        loop {
            let extension = read_null_terminated_string(&mut tree_data);
            if extension.is_empty() {
                break;
            }
            let entries_by_parent_file_stem = entries_by_extension_parent_file_stem
                .entry(CanonicalPathBuf::from_string(extension.to_string()).unwrap())
                .or_default();
            loop {
                let parent = read_null_terminated_string(&mut tree_data);
                if parent.is_empty() {
                    break;
                }
                let entries_by_file_stem = entries_by_parent_file_stem
                    .entry(CanonicalPathBuf::from_string(parent.to_string()).unwrap())
                    .or_default();
                loop {
                    let file_stem = read_null_terminated_string(&mut tree_data);
                    if file_stem.is_empty() {
                        break;
                    }

                    let crc = tree_data.read_u32::<LittleEndian>().unwrap();
                    let preload_bytes = tree_data.read_u16::<LittleEndian>().unwrap();
                    let archive_index = tree_data.read_u16::<LittleEndian>().unwrap();
                    let entry_offset = tree_data.read_u32::<LittleEndian>().unwrap();
                    let entry_length = tree_data.read_u32::<LittleEndian>().unwrap();
                    let terminator = tree_data.read_u16::<LittleEndian>().unwrap();
                    assert_eq!(terminator, 0xffff);
                    let preload_offset =
                        (unsafe { tree_data.as_ptr().offset_from(index_data.as_ptr()) }) as usize;
                    entries_by_file_stem.insert(
                        CanonicalPathBuf::from_string(file_stem.to_string()).unwrap(),
                        DirectoryEntry {
                            _crc: crc,
                            preload_offset,
                            preload_bytes,
                            archive_index,
                            entry_offset,
                            entry_length,
                        },
                    );

                    tree_data = &tree_data[preload_bytes as usize..];
                }
            }
        }

        Ok(Self {
            base_name,
            index_data,
            entries_by_extension_parent_file_stem,
            locked: RefCell::new(VpkLocked {
                path,
                archives: HashMap::new(),
            }),
        })
    }

    fn get_archive<'a>(
        archives: &'a mut HashMap<u16, Mmap>,
        path: &mut PathBuf,
        base_name: &str,
        index: u16,
    ) -> Result<&'a Mmap> {
        archives
            .entry(index)
            .or_try_insert_with(|| -> Result<Mmap> {
                let file_name = format!("{}_{:03}.vpk", base_name, index);
                path.push(file_name);
                let index_file = File::open(path.as_path()).unwrap();
                path.pop();
                Ok(unsafe { Mmap::map(&index_file) }.unwrap())
            })
            .map(|mmap| &*mmap)
    }
}

impl FileLoader for Vpk {
    fn load_file(&self, path: &VpkPath) -> Result<Option<Vec<u8>>> {
        self.entries_by_extension_parent_file_stem
            .get(path.extension())
            .and_then(|entries_by_parent_file_stem| entries_by_parent_file_stem.get(path.parent()))
            .and_then(|entries_by_file_stem| entries_by_file_stem.get(path.file_stem()))
            .map(|entry| {
                let mut data =
                    Vec::with_capacity(entry.preload_bytes as usize + entry.entry_length as usize);

                data.extend_from_slice(
                    &self.index_data
                        [entry.preload_offset..entry.preload_offset + entry.preload_bytes as usize],
                );

                assert!(entry.archive_index != 0x7fff);
                let mut locked = self.locked.borrow_mut();
                let locked = &mut *locked;
                data.extend_from_slice(
                    &Self::get_archive(
                        &mut locked.archives,
                        &mut locked.path,
                        &self.base_name,
                        entry.archive_index,
                    )?[entry.entry_offset as usize
                        ..entry.entry_offset as usize + entry.entry_length as usize],
                );

                Ok(data)
            })
            .transpose()
    }
}

#[derive(Debug)]
#[repr(C)]
struct HeaderV1 {
    signature: u32,
    version: u32,
}

#[derive(Debug)]
#[repr(C)]
struct HeaderV2 {
    signature: u32,
    version: u32,
    tree_size: u32,
    file_data_section_size: u32,
    archive_md5_section_size: u32,
    other_md5_section_size: u32,
    signature_section_size: u32,
}

struct DirectoryEntry {
    _crc: u32,
    preload_offset: usize,
    preload_bytes: u16,
    archive_index: u16,
    entry_offset: u32,
    entry_length: u32,
}
