use core::sync::atomic::Ordering;

use gamecube_mmio::video_interface::VideoInterface;

use crate::os_globals::OS_GLOBALS;

#[no_mangle]
extern "C" fn handle_external_interrupt() {
    OS_GLOBALS.vi_interrupt_fired.store(true, Ordering::Relaxed);

    // TODO: Figure out what ownership means when considering MMIO registers that need to be
    // accessed to service interrupts.
    let mut vi = unsafe { VideoInterface::new_unchecked() };
    vi.modify_display_interrupt_0(|interrupt| interrupt.with_interrupt_status(false));
    vi.modify_display_interrupt_1(|interrupt| interrupt.with_interrupt_status(false));
}
