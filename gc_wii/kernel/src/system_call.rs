use core::arch::asm;
use core::fmt::Write;
use core::{slice, str};

use gamecube_mmio::Uninterruptible;

use crate::os_globals::TEXT_CONSOLE;
use crate::FRAMEBUFFER;

pub fn text_console_print(s: &str) -> u32 {
    let result: u32;
    unsafe {
        asm!(
            "sc",
            out("r0") _,
            inout("r3") 0 => result,
            in("r4") s.as_ptr(),
            in("r5") s.len(),
            out("r6") _,
            out("r7") _,
            out("r8") _,
            out("r9") _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            out("r14") _,
            out("r15") _,
            out("r16") _,
            out("r17") _,
            out("r18") _,
            out("r19") _,
            out("r20") _,
            out("r21") _,
            out("r22") _,
            out("r23") _,
            out("r24") _,
            out("r25") _,
            out("r26") _,
            out("r27") _,
            out("r28") _,
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
        _ => panic!(),
    }
}
