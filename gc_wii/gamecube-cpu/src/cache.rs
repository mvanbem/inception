use core::arch::asm;

/// Invalidates the 32-byte block from the CPU instruction cache that contains the given address.
pub fn invalidate_instruction_cache_block(ptr: *const ()) {
    // SAFETY: Invalidating the instruction cache should be invisible from the CPU's perspective.
    unsafe {
        asm!(
            "icbi 0,{r}",
            r = in(reg) ptr,
            options(preserves_flags, nostack),
        )
    }
}

/// Flushes the 32-byte block from the CPU data cache that contains the given address.
pub fn flush_data_cache_block(ptr: *const ()) {
    // SAFETY: Flushing the data cache should be invisible from the CPU's perspective.
    unsafe {
        asm!(
            "dcbf 0,{r}",
            r = in(reg) ptr,
            options(preserves_flags, nostack),
        )
    }
}

/// Flushes a sequence of 32-byte blocks from the CPU data cache starting with the given
/// address.
pub fn flush_data_cache_blocks(ptr: *const (), block_count: usize) {
    let mut ptr = ptr as usize;
    for _ in 0..block_count {
        // SAFETY: Flushing the data cache should be invisible from the CPU's perspective.
        unsafe {
            asm!(
                "dcbf 0,{r}",
                r = in(reg) ptr,
                options(preserves_flags, nostack),
            )
        }
        ptr += 32;
    }
}
