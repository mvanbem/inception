use crate::registers::msr::*;

/// Disables external interrupts and returns whether they were enabled.
///
/// # Safety
///
/// ???
#[inline(always)]
pub unsafe fn disable_external_interrupts() -> bool {
    let old_msr = mfmsr();
    mtmsr(old_msr.with_external_interrupts_enabled(false));
    old_msr.external_interrupts_enabled()
}

/// Enables external interrupts.
///
/// # Safety
///
/// ???
#[inline(always)]
pub unsafe fn enable_external_interrupts() {
    mtmsr(mfmsr().with_external_interrupts_enabled(true));
}

/// Invokes a function with external interrupts disabled. The previous enabled state is restored
/// after it returns successfully.
///
/// # Safety
///
/// ???
#[inline(always)]
pub unsafe fn with_external_interrupts_disabled<T>(f: impl FnOnce() -> T) -> T {
    let was_enabled = unsafe { disable_external_interrupts() };
    let result = f();
    if was_enabled {
        unsafe { enable_external_interrupts() };
    }
    result
}
