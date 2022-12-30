use core::marker::PhantomData;

use gamecube_cpu::interrupts::with_interrupts_disabled;

/// A token proving that the execution context is uninterruptible for the purpose of MMIO device
/// access.
///
/// The existence of an `Uninterruptible` token means that external interrupts are disabled. This is
/// sufficient to ensure exclusive access to any MMIO devices, protecting read-modify-write
/// sequences from interruption by kernel device drivers.
///
/// It is possible to raise other exceptions, but their handlers in Inception are either
/// unrecoverable (like machine check exceptions) or do not touch MMIO devices (like DSI
/// exceptions).
#[derive(Clone, Copy)]
pub struct Uninterruptible<'a> {
    _phantom_lifetime: PhantomData<&'a ()>,
}

impl<'a> Uninterruptible<'a> {
    /// # Safety
    ///
    /// External interrupts must be disabled while `'a` is live.
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _phantom_lifetime: PhantomData,
        }
    }
}

/// Runs a function uninterruptibly,
pub fn uninterruptible<T>(f: impl FnOnce(Uninterruptible) -> T) -> T {
    unsafe { with_interrupts_disabled(|| f(Uninterruptible::new_unchecked())) }
}
