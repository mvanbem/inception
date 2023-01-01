use core::arch::asm;
use core::{slice, str};

use crate::driver;
use crate::driver::timer::Timestamp;
use crate::thread::{sleep_current_thread_until, suspend_current_thread, WaitingFor};

pub fn text_console_print(s: &str) {
    unsafe {
        asm!(
            "sc",
            in("r3") 0,
            in("r4") s.as_ptr(),
            in("r5") s.len(),
        );
    }
}

pub fn wait_for(waiting_for: WaitingFor) {
    unsafe {
        asm!(
            "sc",
            in("r3") 1,
            in("r4") waiting_for.as_u32(),
        );
    }
}

pub fn sleep_until(wait_time: Timestamp) {
    unsafe {
        asm!(
            "sc",
            in("r3") 2,
            in("r4") (wait_time.0 >> 32) as u32,
            in("r5") wait_time.0 as u32,
        );
    }
}

#[no_mangle]
extern "C" fn handle_system_call(arg1: usize, arg2: usize, arg3: usize) {
    match arg1 {
        0 => {
            // text_console_print
            // TODO: If any of this fails, terminate the offending thread or process.
            let s =
                str::from_utf8(unsafe { slice::from_raw_parts(arg2 as *const u8, arg3) }).unwrap();
            driver::text_console::print(s);
        }
        1 => {
            // wait_for
            suspend_current_thread(WaitingFor::from_u32(arg2 as u32));
        }
        2 => {
            // sleep_until
            sleep_current_thread_until(Timestamp(((arg2 as u64) << 32) | arg3 as u64));
        }
        _ => panic!(),
    }
}
