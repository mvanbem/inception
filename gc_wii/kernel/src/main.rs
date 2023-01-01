#![feature(asm_experimental_arch)]
#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]

use gamecube_cpu::registers::msr::modify_msr;
use gamecube_mmio::processor_interface::{InterruptMask, Interrupts, ProcessorInterface};
use panic_abort as _;

use crate::exception::install_exception_handlers;
use crate::thread::{create_thread, enter_threading, USER_MACHINE_STATE};

mod bsod;
mod driver;
mod exception;
mod external_interrupt;
mod paging;
mod system_call;
mod text_console;
mod thread;

#[no_mangle]
unsafe extern "C" fn main() -> ! {
    // This should be the default state, but set it to be sure.
    modify_msr(|reg| reg.with_external_interrupts_enabled(false));

    install_exception_handlers();

    // Initialize drivers.
    driver::timer::init();
    driver::text_console::init();

    // Prepare the initial threads.
    create_thread(crate::user::entry_a, USER_MACHINE_STATE, None);
    create_thread(crate::user::entry_b, USER_MACHINE_STATE, None);

    driver::text_console::print("System initialized. Starting threads.\n");

    ProcessorInterface::new().write_interrupt_mask(
        InterruptMask::zero().with_interrupts(Interrupts::zero().with_video_interface(true)),
    );

    enter_threading();
}

mod user {
    use core::fmt::Write;

    use arrayvec::ArrayString;
    use gamecube_cpu::registers::time_base;

    use crate::driver::timer::Timestamp;
    use crate::system_call;

    #[no_mangle]
    pub extern "C" fn entry_a() -> ! {
        const INTERVAL_TICKS: u64 = 333_333 * 81 / 2;
        let mut wait_time = Timestamp(time_base() + INTERVAL_TICKS);
        loop {
            system_call::sleep_until(wait_time);
            wait_time.0 += INTERVAL_TICKS;

            // Print.
            let mut buf = ArrayString::<1024>::new();
            writeln!(
                &mut buf,
                "Thread A here! It's been a third of a second. Sleeping until 0x{:016x}",
                wait_time.0,
            )
            .unwrap();
            system_call::text_console_print(buf.as_str());
        }
    }

    #[no_mangle]
    pub extern "C" fn entry_b() -> ! {
        const INTERVAL_TICKS: u64 = 500_000 * 81 / 2;
        let mut wait_time = Timestamp(time_base() + INTERVAL_TICKS);
        loop {
            system_call::sleep_until(wait_time);
            wait_time.0 += INTERVAL_TICKS;

            // Print.
            let mut buf = ArrayString::<1024>::new();
            writeln!(
                &mut buf,
                "I am thread B. Half a second elapsed. Sleeping until {:016x}",
                wait_time.0
            )
            .unwrap();
            system_call::text_console_print(buf.as_str());
        }
    }
}
