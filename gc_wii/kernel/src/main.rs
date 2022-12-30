#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]

use core::fmt::Write;
use core::sync::atomic::Ordering;

use gamecube_cpu::registers::time_base;
use gamecube_mmio::processor_interface::{InterruptMask, Interrupts};
use gamecube_video_driver::framebuffer::Framebuffer;
use gamecube_video_driver::VideoDriver;
use panic_abort as _;

mod bsod;
mod external_interrupt;
mod init;
mod os_globals;
mod paging;
mod text_console;

use crate::os_globals::OS_GLOBALS;
use crate::text_console::TextConsole;

// Large buffers.
#[link_section = ".bss"]
static FRAMEBUFFER: Framebuffer = Framebuffer::zero();

#[no_mangle]
extern "C" fn main() -> ! {
    let init::Devices { pi, vi, .. } = unsafe { init::init() };

    let mut video = VideoDriver::new(vi);
    video.configure_for_ntsc_480p(FRAMEBUFFER.as_ptr().cast());

    // Enable VI interrupts.
    pi.write_interrupt_mask(
        InterruptMask::zero().with_interrupts(Interrupts::zero().with_video_interface(true)),
    );

    let mut console = TextConsole::new();
    let mut counter = 0u32;
    let mut last_time = 0;
    loop {
        if OS_GLOBALS.vi_interrupt_fired.load(Ordering::Relaxed) {
            OS_GLOBALS
                .vi_interrupt_fired
                .store(false, Ordering::Relaxed);
            let start_time = time_base();
            if console.modified() {
                console.render(&FRAMEBUFFER);
            }
            let end_time = time_base();

            writeln!(
                &mut console,
                "Text console render time: {} us",
                (2 * (end_time - start_time) / 81) as u32,
            )
            .unwrap();

            writeln!(
                &mut console,
                "Frame time: {} us",
                (2 * (end_time - last_time) / 81) as u32,
            )
            .unwrap();
            last_time = end_time;

            writeln!(&mut console, "Counter: {counter}").unwrap();
            counter = counter.wrapping_add(1);
        }
    }
}
