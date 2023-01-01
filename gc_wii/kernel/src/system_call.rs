use core::arch::asm;
use core::fmt::Write;
use core::{slice, str};

use gamecube_mmio::Uninterruptible;

use crate::external_interrupt::Interrupt;
use crate::os_globals::TEXT_CONSOLE;
use crate::thread::suspend_current_thread;
use crate::FRAMEBUFFER;

pub fn text_console_print(s: &str) -> u32 {
    let result: u32;
    unsafe {
        asm!(
            "sc",
            inout("r3") 0 => result,
            in("r4") s.as_ptr(),
            in("r5") s.len(),
        );
    }
    result
}

pub fn wait_for(all: Interrupt) -> u32 {
    let result: u32;
    unsafe {
        asm!(
            "sc",
            inout("r3") 1 => result,
            in("r4") all.as_u32(),
        );
    }
    result
}

#[no_mangle]
extern "C" fn handle_system_call(arg1: usize, arg2: usize, arg3: usize) {
    // SAFETY: This is an interrupt handler and interrupts are disabled.
    let _u = unsafe { Uninterruptible::new_unchecked() };

    match arg1 {
        0 => {
            // text_console_print
            // TODO: If any of this fails, terminate the offending thread or process.
            let s =
                str::from_utf8(unsafe { slice::from_raw_parts(arg2 as *const u8, arg3) }).unwrap();
            let text_console = unsafe { &mut TEXT_CONSOLE };
            write!(text_console, "{}", s).unwrap();
            text_console.render(&FRAMEBUFFER);
        }
        1 => {
            // wait_for
            suspend_current_thread(Interrupt::from_u32(arg2 as u32));
        }
        _ => panic!(),
    }
}
