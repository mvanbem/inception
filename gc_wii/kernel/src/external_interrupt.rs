use core::sync::atomic::Ordering;

use gamecube_mmio::permission::PermissionRoot;
use gamecube_mmio::uninterruptible::Uninterruptible;
use gamecube_mmio::video_interface::VideoInterface;

use crate::os_globals::OS_GLOBALS;

#[no_mangle]
extern "C" fn handle_external_interrupt() {
    // SAFETY: This is an interrupt handler and interrupts are disabled.
    let root = unsafe { PermissionRoot::new_unchecked() };
    let u = unsafe { Uninterruptible::new_unchecked() };

    OS_GLOBALS.vi_interrupt_fired.store(true, Ordering::Relaxed);

    let vi = VideoInterface::new(root);
    vi.modify_display_interrupt_0(u, |reg| reg.with_interrupt_status(false));
    vi.modify_display_interrupt_1(u, |reg| reg.with_interrupt_status(false));
}
