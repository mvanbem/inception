#![feature(asm_experimental_arch)]
#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]

use core::fmt::Write;

use gamecube_cpu::registers::msr::modify_msr;
use gamecube_mmio::processor_interface::{InterruptMask, Interrupts};
use gamecube_video_driver::framebuffer::Framebuffer;
use gamecube_video_driver::VideoDriver;
use panic_abort as _;

use crate::os_globals::TEXT_CONSOLE;
use crate::thread::{create_thread, enter_threading, USER_MACHINE_STATE};

mod bsod;
mod external_interrupt;
mod init;
mod os_globals;
mod paging;
mod system_call;
mod text_console;
mod thread;

// Large buffers.
#[link_section = ".bss"]
static FRAMEBUFFER: Framebuffer = Framebuffer::zero();

#[no_mangle]
extern "C" fn main() -> ! {
    let init::Devices { pi, vi, .. } = unsafe { init::init() };

    let mut video = VideoDriver::new(vi);
    video.configure_for_ntsc_480p(FRAMEBUFFER.as_ptr().cast());

    {
        let text_console = unsafe { &mut TEXT_CONSOLE };
        writeln!(text_console, "System initialized. Starting user code.").unwrap();
        text_console.render(&FRAMEBUFFER);
    }

    create_thread(crate::user::entry_a, USER_MACHINE_STATE, None);
    create_thread(crate::user::entry_b, USER_MACHINE_STATE, None);

    // Enable interrupts and hand off to threads.
    pi.write_interrupt_mask(
        InterruptMask::zero().with_interrupts(Interrupts::zero().with_video_interface(true)),
    );
    unsafe {
        modify_msr(|reg| {
            reg.with_external_interrupts_enabled(true)
                .with_machine_check_enabled(true)
        });
        enter_threading();
    }
}

mod user {
    use core::fmt::Write;

    use arrayvec::ArrayString;
    use gamecube_cpu::registers::time_base;

    use crate::external_interrupt::{Interrupt, VideoInterfaceInterrupt};
    use crate::system_call;

    #[no_mangle]
    pub extern "C" fn entry_a() -> ! {
        const INTERVAL_TICKS: u64 = 333_333 * 81 / 2;
        let mut wait_time = time_base() + INTERVAL_TICKS;
        loop {
            // Wait.
            loop {
                let now = time_base();
                if now >= wait_time {
                    wait_time = now + INTERVAL_TICKS;
                    break;
                }
                system_call::wait_for(Interrupt::zero().with_video_interface(
                    VideoInterfaceInterrupt::zero().with_display_interrupt_0(true),
                ));
            }

            // Print.
            let mut buf = ArrayString::<1024>::new();
            writeln!(&mut buf, "Thread A here! It's been a third of a second.").unwrap();
            system_call::text_console_print(buf.as_str());
        }
    }

    #[no_mangle]
    pub extern "C" fn entry_b() -> ! {
        const INTERVAL_TICKS: u64 = 500_000 * 81 / 2;
        let mut wait_time = time_base() + INTERVAL_TICKS;
        loop {
            // Wait.
            loop {
                let now = time_base();
                if now >= wait_time {
                    wait_time = now + INTERVAL_TICKS;
                    break;
                }
                system_call::wait_for(Interrupt::zero().with_video_interface(
                    VideoInterfaceInterrupt::zero().with_display_interrupt_0(true),
                ));
            }

            // Print.
            let mut buf = ArrayString::<1024>::new();
            writeln!(&mut buf, "I am thread B. Half a second elapsed.").unwrap();
            system_call::text_console_print(buf.as_str());
        }
    }
}
