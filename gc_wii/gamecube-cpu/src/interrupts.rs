use crate::registers::msr::*;

/// Disables external interrupts and returns whether they were enabled.
#[inline(always)]
pub unsafe fn disable_interrupts() -> bool {
    let old_msr = mfmsr();
    mtmsr(old_msr.with_external_interrupts_enabled(false));
    old_msr.external_interrupts_enabled()
}

/// Enables external interrupts.
#[inline(always)]
pub unsafe fn enable_interrupts() {
    mtmsr(mfmsr().with_external_interrupts_enabled(true));
}

/// Invokes a function with external interrupts disabled. The previous enabled state is restored
/// after it returns successfully.
#[inline(always)]
pub fn with_interrupts_disabled<T>(f: impl FnOnce() -> T) -> T {
    let was_enabled = unsafe { disable_interrupts() };
    let result = f();
    if was_enabled {
        unsafe { enable_interrupts() };
    }
    result
}
