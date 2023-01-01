use gamecube_mmio::processor_interface::ProcessorInterface;
use gamecube_mmio::video_interface::VideoInterface;
use mvbitfield::prelude::*;

use crate::driver;
use crate::thread::{wake_thread, WaitingFor};

#[no_mangle]
extern "C" fn handle_external_interrupt() -> WaitingFor {
    let pi = ProcessorInterface::new();
    let vi = VideoInterface::new();

    let mut fired = WaitingFor::zero();
    let pi_cause = pi.read_interrupt_cause();
    if pi_cause.interrupts().video_interface() {
        for i in 0..4 {
            let i = U2::new(i).unwrap();
            let reg = vi.read_display_interrupt(i);
            if reg.interrupt_status() {
                // NOTE: This could in principle lose an interrupt. I don't know what the timing
                // requirements would be. One VI interrupt per field should be safe.
                vi.write_display_interrupt(i, reg.with_interrupt_status(false));

                match i.as_u8() {
                    0 => fired.modify_video_interface(|reg| reg.with_display_interrupt_0(true)),
                    1 => fired.modify_video_interface(|reg| reg.with_display_interrupt_1(true)),
                    2 => fired.modify_video_interface(|reg| reg.with_display_interrupt_2(true)),
                    3 => fired.modify_video_interface(|reg| reg.with_display_interrupt_3(true)),
                    _ => unreachable!(),
                }
            }
        }
    }

    driver::timer::for_each_elapsed(|thread_id| {
        wake_thread(thread_id, WaitingFor::zero().with_timer(true));
    });

    fired
}
