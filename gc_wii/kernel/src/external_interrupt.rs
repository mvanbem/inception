use core::sync::atomic::Ordering;

use gamecube_mmio::processor_interface::ProcessorInterface;
use gamecube_mmio::video_interface::VideoInterface;
use gamecube_mmio::{PermissionRoot, Uninterruptible};
use mvbitfield::mvbitfield;
use mvbitfield::prelude::U2;

use crate::os_globals::VI_INTERRUPT_FIRED;

mvbitfield! {
    pub struct Interrupt: u32 {
        pub video_interface: 4 as VideoInterfaceInterrupt,
    }
}

mvbitfield! {
    pub struct VideoInterfaceInterrupt: U4 {
        pub display_interrupt_0: 1 as bool,
        pub display_interrupt_1: 1 as bool,
        pub display_interrupt_2: 1 as bool,
        pub display_interrupt_3: 1 as bool,
    }
}

#[no_mangle]
extern "C" fn handle_external_interrupt() -> Interrupt {
    // SAFETY: This is an interrupt handler and interrupts are disabled.
    let root = unsafe { PermissionRoot::new_unchecked() };
    let _u = unsafe { Uninterruptible::new_unchecked() };
    let pi = ProcessorInterface::new(root);
    let vi = VideoInterface::new(root);

    let mut fired = Interrupt::zero();
    let pi_cause = pi.read_interrupt_cause();
    if pi_cause.interrupts().video_interface() {
        VI_INTERRUPT_FIRED.store(true, Ordering::Relaxed);

        for i in 0..4 {
            let i = U2::new(i).unwrap();
            let reg = vi.read_display_interrupt(i);
            if reg.interrupt_status() {
                // NOTE: This could in principle lose an interrupt. I don't know what the timing
                // requirements would be. One VI interrupt per field should be safe.
                vi.write_display_interrupt(i, reg.with_interrupt_status(false));

                match i.as_u8() {
                    0 => fired.modify_video_interface(|reg| reg.with_display_interrupt_0(true)),
                    1 => fired.modify_video_interface(|reg| reg.with_display_interrupt_0(true)),
                    2 => fired.modify_video_interface(|reg| reg.with_display_interrupt_0(true)),
                    3 => fired.modify_video_interface(|reg| reg.with_display_interrupt_0(true)),
                    _ => unreachable!(),
                }
            }
        }
    }

    fired
}
