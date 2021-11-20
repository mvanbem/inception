#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![no_std]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::panic::PanicInfo;

use alloc::format;
use alloc::string::String;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Cast cached virtual address to uncached virtual address, e.g. `0x8xxxxxxx` -> `0xCxxxxxxx`
pub fn MEM_K0_TO_K1<T>(x: *mut T) -> *mut T {
    (x as usize - SYS_BASE_CACHED as usize + SYS_BASE_UNCACHED as usize) as *mut T
}

#[global_allocator]
static ALLOCATOR: LibogcAllocator = LibogcAllocator;

struct LibogcAllocator;

unsafe impl GlobalAlloc for LibogcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        libc::malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        libc::free(ptr as *mut c_void);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = libc::malloc(layout.size());
        libc::memset(ptr, 0, layout.size());
        ptr as *mut u8
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        libc::realloc(ptr as *mut c_void, new_size) as *mut u8
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        let rmode = VIDEO_GetPreferredMode(core::ptr::null_mut());
        CON_InitEx(
            rmode,
            16,
            16,
            (*rmode).fbWidth as i32 - 32,
            (*rmode).xfbHeight as i32 - 32,
        );
        if let Some(location) = info.location() {
            let buf = format!("{}\0", location);
            libc::printf(b"Panic at %s:\n\0".as_ptr(), buf.as_ptr());
        } else {
            libc::printf(b"Panic! (no location information)\n\0".as_ptr());
        }

        if let Some(payload) = info.payload().downcast_ref::<&str>().copied() {
            let buf = format!("{}\0", payload);
            libc::printf(b"%s\n\0".as_ptr(), buf);
        } else if let Some(payload) = info.payload().downcast_ref::<String>() {
            let buf = format!("{}\0", payload);
            libc::printf(b"%s\n\0".as_ptr(), buf);
        }

        libc::printf(b"Press Start to exit to the loader.\0".as_ptr());

        // Wait for the player to press Start.
        loop {
            PAD_ScanPads();
            if (PAD_ButtonsHeld(0) & PAD_BUTTON_START as u16) != 0 {
                libc::exit(0);
            }

            VIDEO_WaitVSync();
        }
    }
}