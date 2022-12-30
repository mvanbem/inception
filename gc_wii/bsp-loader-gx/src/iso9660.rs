#![allow(dead_code)]

use core::mem::size_of;
use core::ops::{ControlFlow, Range};

use aligned::{Aligned, A32};
use alloc::format;
use alloc::vec::Vec;
use bytemuck::{from_bytes, Pod, Zeroable};
use gamecube_dvd_driver::DvdDriver;
use ogc_sys::GlobalAlign32;

pub struct DiscReader<'reg> {
    dvd: DvdDriver<'reg>,
    path_table_offset: usize,
    path_table_size: usize,
    root_directory_extent: Range<usize>,
}

impl<'reg> DiscReader<'reg> {
    pub fn new(mut dvd: DvdDriver<'reg>) -> Self {
        let mut buf: Aligned<A32, _> = Aligned([0; 2048]);
        for i in 16.. {
            dvd.read(2048 * i, &mut *buf).unwrap();

            let volume_descriptor: &VolumeDescriptorHeader =
                from_bytes(&(*buf)[..size_of::<VolumeDescriptorHeader>()]);
            match volume_descriptor.type_ {
                // Primary volume descriptor.
                1 => {
                    let primary: &PrimaryVolumeDescriptor =
                        from_bytes(&(*buf)[..size_of::<PrimaryVolumeDescriptor>()]);
                    assert_eq!(primary.version, 1);
                    assert_eq!(primary.logical_block_size_be, 2048);
                    return Self {
                        dvd,
                        path_table_offset: 2048 * primary.be_path_table_lba_be as usize,
                        path_table_size: primary.path_table_size_be as usize,
                        root_directory_extent: primary.root_directory_entry.extent(),
                    };
                }

                // Terminator.
                255 => unreachable!(),

                // Some other volume descriptor.
                _ => continue,
            }
        }
        unreachable!();
    }

    fn scan_path_table<R>(
        &mut self,
        mut f: impl FnMut(u16, &BePathTableEntry) -> ControlFlow<R>,
    ) -> Option<R> {
        let mut offset = self.path_table_offset;
        let mut size_remaining = self.path_table_size;
        let mut buf: Aligned<A32, _> = Aligned([0; 1024]);
        let mut index = 0;
        while size_remaining > 0 {
            let read_offset = offset & !3;
            let struct_offset = offset - read_offset;
            let size = (buf
                .len()
                .min(BePathTableEntry::MAX_SIZE)
                .min(size_remaining)
                + 31)
                & !31;

            self.dvd.read(read_offset, &mut (*buf)[..size]).unwrap();
            let entry: &BePathTableEntry =
                from_bytes(&(*buf)[struct_offset..struct_offset + size_of::<BePathTableEntry>()]);
            match f(index, entry) {
                ControlFlow::Continue(()) => (),
                ControlFlow::Break(result) => return Some(result),
            }

            let size = entry.size();
            offset += size;
            size_remaining -= size;
            index += 1;
        }

        None
    }

    fn find_directory_with_parent(
        &mut self,
        name: &str,
        parent_index: u16,
    ) -> Option<(u16, usize)> {
        unsafe {
            let buf = format!(
                "find_directory_with_parent({:?}, {})\n\0",
                name, parent_index,
            );
            libc::printf(b"%s\0".as_ptr(), buf.as_ptr());
        }
        self.scan_path_table(|index, entry| {
            unsafe {
                let buf = format!(
                    "    Scanning index={}, entry={:?}, name={:?}\n\0",
                    index,
                    entry,
                    entry.name(),
                );
                libc::printf(b"%s\0".as_ptr(), buf.as_ptr());
            }
            if entry.parent_index == parent_index && entry.name().eq_ignore_ascii_case(name) {
                unsafe { libc::printf(b"    Match!\n\0".as_ptr()) };
                return ControlFlow::Break((index, entry.extent() as usize));
            }
            ControlFlow::Continue(())
        })
    }

    fn for_each_directory_entry<R>(
        dvd: &mut DvdDriver<'reg>,
        extent: Range<usize>,
        mut f: impl FnMut(&mut DvdDriver<'reg>, &DirectoryEntry) -> ControlFlow<R>,
    ) -> Option<R> {
        // Directory entries never cross a sector boundary, so read entire sectors and scan them.
        let sector_start = extent.start & !2047;
        let mut skip = 2; // Each directory starts with `.` and `..` entries.
        for sector_start in (sector_start..extent.end).step_by(2048) {
            let mut sector_data: Aligned<A32, _> = Aligned([0; 2048]);
            dvd.read(sector_start, &mut *sector_data).unwrap();

            // Scan directory entries within this sector.
            let mut struct_offset = extent.start.checked_sub(sector_start).unwrap_or(0);
            let end = (extent.end - sector_start).min(2048);
            while struct_offset + size_of::<DirectoryEntry>() <= end {
                let entry: &DirectoryEntry = from_bytes(
                    &(*sector_data)[struct_offset..struct_offset + size_of::<DirectoryEntry>()],
                );
                if entry.record_len == 0 {
                    // That's all for this sector.
                    break;
                }

                if skip == 0 {
                    match f(dvd, entry) {
                        ControlFlow::Continue(()) => (),
                        ControlFlow::Break(result) => return Some(result),
                    }
                } else {
                    skip -= 1;
                }

                struct_offset += entry.record_len as usize;
            }
        }

        None
    }

    fn list_directory_with_extent(
        dvd: &mut DvdDriver<'reg>,
        path: &str,
        f: &mut impl FnMut(&str),
        extent: Range<usize>,
    ) {
        if path.is_empty() {
            // Leaf case. List this directory.
            Self::for_each_directory_entry(dvd, extent, |_dvd, entry| {
                f(entry.rock_ridge_name().unwrap_or(entry.name()));
                ControlFlow::<()>::Continue(())
            });
        } else {
            // Traversal case. Look for the named directory and recurse into it.
            let name = path.split('/').next().unwrap();
            Self::for_each_directory_entry(dvd, extent, |dvd, entry| {
                if entry
                    .rock_ridge_name()
                    .map(|entry_name| entry_name.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
                {
                    // Match.
                    assert_eq!(entry.flags & 2, 2); // Must be a directory.
                    Self::list_directory_with_extent(
                        dvd,
                        path.splitn(2, '/').skip(1).next().unwrap_or(""),
                        f,
                        entry.extent(),
                    );
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            });
        }
    }

    pub fn list_directory(&mut self, path: &str, mut f: impl FnMut(&str)) {
        Self::list_directory_with_extent(
            &mut self.dvd,
            path,
            &mut f,
            self.root_directory_extent.clone(),
        );
    }

    fn read_file_with_extent(
        dvd: &mut DvdDriver<'reg>,
        path: &str,
        extent: Range<usize>,
    ) -> Vec<u8, GlobalAlign32> {
        let (name, sub_path) = {
            let mut iter = path.splitn(2, '/');
            let name = iter.next().unwrap();
            let sub_path = iter.next().unwrap_or("");
            (name, sub_path)
        };

        Self::for_each_directory_entry(dvd, extent, |dvd, entry| {
            if entry
                .rock_ridge_name()
                .map(|entry_name| entry_name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
            {
                // Match.
                if sub_path.is_empty() {
                    // Read the file.
                    let disc_offset = entry.extent_start();
                    assert_eq!(disc_offset & 31, 0);
                    let size = entry.extent_size();
                    let alloc_size = (size + 31) & !31;
                    let mut data = Vec::with_capacity_in(alloc_size, GlobalAlign32);

                    dvd.read_maybe_uninit(disc_offset, data.spare_capacity_mut())
                        .unwrap();
                    unsafe { data.set_len(size) }

                    ControlFlow::Break(data)
                } else {
                    // Recurse into the directory.
                    assert_eq!(entry.flags & 2, 2); // Must be a directory.
                    ControlFlow::Break(Self::read_file_with_extent(dvd, sub_path, entry.extent()))
                }
            } else {
                ControlFlow::Continue(())
            }
        })
        .unwrap()
    }

    pub fn read_file(&mut self, path: &str) -> Vec<u8, GlobalAlign32> {
        Self::read_file_with_extent(&mut self.dvd, path, self.root_directory_extent.clone())
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct VolumeDescriptorHeader {
    type_: u8,
    identifier: [u8; 5],
    version: u8,
    boot_system_identifier: [u8; 32],
    boot_identifier: [u8; 32],
    // boot_system_use (not Pod/Zeroable at the moment...)
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct PrimaryVolumeDescriptor {
    type_: u8,
    identifier: [u8; 5],
    version: u8,
    _unused1: u8,
    system_identifier: [u8; 32],
    volume_identifier: [u8; 32],
    _unused2: [u8; 8],
    volume_space_size_le: u32,
    volume_space_size_be: u32,
    _unused3: [u8; 32],
    volume_set_size_le: u16,
    volume_set_size_be: u16,
    volume_sequence_number_le: u16,
    volume_sequence_number_be: u16,
    logical_block_size_le: u16,
    logical_block_size_be: u16,
    path_table_size_le: u32,
    path_table_size_be: u32,
    le_path_table_lba_le: u32,
    optional_le_path_table_lba_le: u32,
    be_path_table_lba_be: u32,
    optional_be_path_table_lba_be: u32,
    root_directory_entry: DirectoryEntry,
    _padding: u16,
    // more fields
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct BePathTableEntry {
    name_len: u8,
    extended_attribute_record_len: u8,
    // NOTE: Split into two u16 fields because this struct is only 2-byte aligned!
    extent_hi: u16,
    extent_lo: u16,
    parent_index: u16,
    name: [u8; 0],
}

impl BePathTableEntry {
    const MAX_SIZE: usize = 256 + 8;

    fn size(&self) -> usize {
        ((self.name_len as usize + 1) & !1) + 8
    }

    fn extent(&self) -> u32 {
        (self.extent_hi as u32) << 16 | self.extent_lo as u32
    }

    fn name(&self) -> &str {
        let name_ptr: *const u8 = self.name.as_ptr();
        let name_len = unsafe { libc::strlen(name_ptr) };
        let name_slice = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
        core::str::from_utf8(name_slice).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct DirectoryEntry {
    record_len: u8,
    extended_attribute_record_len: u8,
    // NOTE: Split into two u16 fields because this struct is only 2-byte aligned!
    extent_start_le_lo: u16,
    extent_start_le_hi: u16,
    // NOTE: Split into two u16 fields because this struct is only 2-byte aligned!
    extent_start_be_hi: u16,
    extent_start_be_lo: u16,
    // NOTE: Split into two u16 fields because this struct is only 2-byte aligned!
    extent_size_le_lo: u16,
    extent_size_le_hi: u16,
    // NOTE: Split into two u16 fields because this struct is only 2-byte aligned!
    extent_size_be_hi: u16,
    extent_size_be_lo: u16,
    date_year: u8,
    date_month: u8,
    date_day: u8,
    date_hour: u8,
    date_minute: u8,
    date_second: u8,
    date_gmt_offset: i8,
    flags: u8,
    interleaved_file_unit_size: u8,
    interleaved_gap_size: u8,
    volume_sequence_number_le: u16,
    volume_sequence_number_be: u16,
    name_len: u8,
    name: [u8; 1],
}

impl DirectoryEntry {
    fn extent_start(&self) -> usize {
        (self.extent_start_be_hi as usize) << 27 | (self.extent_start_be_lo as usize) << 11
    }

    fn extent_size(&self) -> usize {
        (self.extent_size_be_hi as usize) << 16 | self.extent_size_be_lo as usize
    }

    fn extent(&self) -> Range<usize> {
        let start = self.extent_start();
        start..start + self.extent_size()
    }

    fn name(&self) -> &str {
        let name_ptr: *const u8 = self.name.as_ptr();
        let name_len = self.name_len as usize;
        let name_slice = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
        core::str::from_utf8(name_slice).unwrap()
    }

    fn padded_name_len(&self) -> usize {
        self.name_len as usize + ((self.name_len & 1) ^ 1) as usize
    }

    unsafe fn as_slice(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const Self as *const u8, self.record_len as usize)
        }
    }

    const NAME_OFFSET: usize = 33;

    fn rock_ridge_name(&self) -> Option<&str> {
        let mut data = &unsafe { self.as_slice() }[Self::NAME_OFFSET + self.padded_name_len()..];
        while data.len() >= size_of::<RockRidgeExtensionHeader>() {
            let header: &RockRidgeExtensionHeader =
                from_bytes(&data[..size_of::<RockRidgeExtensionHeader>()]);
            if header.id == [b'N', b'M'] {
                let rock_ridge_nm: &RockRidgeNm = from_bytes(&data[..size_of::<RockRidgeNm>()]);
                return Some(rock_ridge_nm.name());
            }

            data = &data[header.len as usize..];
        }

        None
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct RockRidgeExtensionHeader {
    id: [u8; 2],
    len: u8,
    version: u8,
    data: [u8; 0],
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct RockRidgeNm {
    header: RockRidgeExtensionHeader,
    flags: u8,
    name: [u8; 0],
}

impl RockRidgeNm {
    fn name(&self) -> &str {
        let name_ptr: *const u8 = self.name.as_ptr();
        let name_len = self.header.len as usize - 5;
        let name_slice = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
        core::str::from_utf8(name_slice).unwrap()
    }
}
