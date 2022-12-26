use core::arch::asm;

/// Flushes a sequence of 32-byte blocks from the CPU data cache starting with the given
/// address.
#[inline(never)]
pub fn flush_data_cache(mut ptr: *const (), block_count: usize) {
    for _ in 0..block_count {
        // SAFETY: Flushing the data cache should be invisible from the CPU's perspective.
        unsafe {
            asm!(
                "dcbf 0,{r}",
                r = in(reg) ptr,
                options(preserves_flags, nostack),
            )
        }
        ptr = unsafe { ptr.byte_offset(32) };
    }
}
