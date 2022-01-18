use core::ffi::c_void;
use core::intrinsics::write_bytes;
use core::mem::size_of;

use aligned::{Aligned, A32};
use bytemuck::Zeroable;

use crate::ipl_interface::OsReportFn;

pub struct AppLoader {
    state: State,
    os_report: OsReportFn,
    boot_header: Aligned<A32, BootHeader>,
    dol_header: Aligned<A32, DolHeader>,
}

enum State {
    FetchBootHeader,
    FetchDolHeader,
    LoadDolSection { index: usize },
    ClearBss,
    Done,
}

pub struct LoadCommand {
    pub addr: *mut c_void,
    pub len: usize,
    pub offset: usize,
}

impl AppLoader {
    pub fn new(os_report: OsReportFn) -> Self {
        Self {
            state: State::FetchBootHeader,
            os_report,
            boot_header: Aligned(Zeroable::zeroed()),
            dol_header: Aligned(Zeroable::zeroed()),
        }
    }

    pub fn main(&mut self) -> Option<LoadCommand> {
        loop {
            match self.state {
                State::FetchBootHeader => {
                    self.state = State::FetchDolHeader;
                    return Some(LoadCommand {
                        addr: &mut *self.boot_header as *mut BootHeader as _,
                        len: (size_of::<BootHeader>() + 31) & !31,
                        offset: BootHeader::FIXED_DISC_OFFSET,
                    });
                }

                State::FetchDolHeader => {
                    unsafe {
                        (self.os_report)(
                            b"main(): got DOL offset %08x\0".as_ptr(),
                            self.boot_header.dol_offset,
                        )
                    }

                    self.state = State::LoadDolSection { index: 0 };
                    return Some(LoadCommand {
                        addr: &mut *self.dol_header as *mut DolHeader as _,
                        len: (size_of::<DolHeader>() + 31) & !31,
                        offset: self.boot_header.dol_offset,
                    });
                }

                State::LoadDolSection { ref mut index } => {
                    while *index < 18 {
                        let section = unsafe { self.dol_header.get_section_unchecked(*index) };
                        *index += 1;
                        if section.size > 0 {
                            let command = LoadCommand {
                                addr: section.load_addr as *mut c_void,
                                len: section.size,
                                offset: self.boot_header.dol_offset + section.offset,
                            };
                            unsafe {
                                (self.os_report)(
                                    b"main(): Loading DOL section %d from %08x+%08x to %08x\0"
                                        .as_ptr(),
                                    *index - 1,
                                    command.offset,
                                    command.len,
                                    command.addr,
                                )
                            }
                            return Some(command);
                        }
                    }
                    self.state = State::ClearBss;
                }

                State::ClearBss => {
                    unsafe {
                        write_bytes(
                            self.dol_header.bss_start as *mut u8,
                            0,
                            self.dol_header.bss_size,
                        )
                    }
                    self.state = State::Done;
                }

                State::Done => {
                    return None;
                }
            }
        }
    }

    pub fn entry_point(&self) -> *const c_void {
        unsafe {
            (self.os_report)(
                b"main(): Entering DOL at %08x\0".as_ptr(),
                self.dol_header.entry_point,
            )
        }
        self.dol_header.entry_point as _
    }
}

#[derive(Clone, Copy, Zeroable)]
#[repr(C)]
struct BootHeader {
    dol_offset: usize,
    _padding: [usize; 7],
}

impl BootHeader {
    const FIXED_DISC_OFFSET: usize = 0x420;
}

#[derive(Clone, Copy, Zeroable)]
#[repr(C)]
struct DolHeader {
    section_offsets: [usize; 18],
    section_load_addrs: [usize; 18],
    section_sizes: [usize; 18],
    bss_start: usize,
    bss_size: usize,
    entry_point: usize,
}

struct DolSection {
    offset: usize,
    load_addr: usize,
    size: usize,
}

impl DolHeader {
    unsafe fn get_section_unchecked(&self, index: usize) -> DolSection {
        DolSection {
            offset: *self.section_offsets.get_unchecked(index),
            load_addr: *self.section_load_addrs.get_unchecked(index),
            size: *self.section_sizes.get_unchecked(index),
        }
    }
}
