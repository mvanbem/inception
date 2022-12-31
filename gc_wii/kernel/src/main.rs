#![feature(asm_experimental_arch)]
#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]

use core::fmt::Write;

use gamecube_cpu::registers::msr::{MachineState, PrivilegeLevel};
use gamecube_mmio::processor_interface::{InterruptMask, Interrupts};
use gamecube_video_driver::framebuffer::Framebuffer;
use gamecube_video_driver::VideoDriver;
use panic_abort as _;

use crate::os_globals::TEXT_CONSOLE;

mod bsod;
mod external_interrupt;
mod init;
mod os_globals;
mod paging;
mod system_call;
mod text_console;

// Large buffers.
#[link_section = ".bss"]
static FRAMEBUFFER: Framebuffer = Framebuffer::zero();

extern "C" {
    fn change_context(srr0: extern "C" fn() -> !, srr1: u32) -> !;
}

#[no_mangle]
extern "C" fn main() -> ! {
    let init::Devices { pi, vi, .. } = unsafe { init::init() };

    let mut video = VideoDriver::new(vi);
    video.configure_for_ntsc_480p(FRAMEBUFFER.as_ptr().cast());

    // Enable VI interrupts.
    pi.write_interrupt_mask(
        InterruptMask::zero().with_interrupts(Interrupts::zero().with_video_interface(true)),
    );

    {
        let text_console = unsafe { &mut TEXT_CONSOLE };
        writeln!(text_console, "System initialized. Starting user code.").unwrap();
        text_console.render(&FRAMEBUFFER);
    }

    unsafe {
        change_context(
            crate::user::entry,
            MachineState::zero()
                .with_exception_is_recoverable(true)
                .with_data_address_translation_enabled(true)
                .with_instruction_address_translation_enabled(true)
                .with_machine_check_enabled(true)
                .with_privilege_level(PrivilegeLevel::User)
                .with_external_interrupts_enabled(true)
                .as_u32(),
        )
    };
}

mod user {
    use core::fmt::Write;

    use arrayvec::ArrayString;
    use gamecube_cpu::registers::time_base;

    use crate::system_call;

    #[no_mangle]
    pub extern "C" fn entry() -> ! {
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
            }

            // Print.
            let mut buf = ArrayString::<1024>::new();
            writeln!(
                &mut buf,
                "This is user code printing once every third of a second."
            )
            .unwrap();
            system_call::text_console_print(buf.as_str());
        }
    }
}
