use core::arch::asm;

pub mod msr;

pub fn time_base() -> u64 {
    loop {
        let tbu1: u32;
        let tbl: u32;
        let tbu2: u32;
        unsafe {
            asm!(
                "mftbu {ru1}",
                "mftb {rl}",
                "mftbu {ru2}",
                ru1 = out(reg) tbu1,
                rl = out(reg) tbl,
                ru2 = out(reg) tbu2,
                options(nomem, preserves_flags, nostack),
            );
        }
        if tbu1 == tbu2 {
            return (tbu1 as u64) << 32 | tbl as u64;
        }
    }
}

pub fn decrementer() -> u32 {
    let result;
    unsafe {
        asm!(
            "mfdec {r}",
            r = out(reg) result,
            options(nomem, preserves_flags, nostack),
        );
    }
    result
}

pub fn set_decrementer(value: u32) {
    unsafe {
        asm!(
            "mtdec {r}",
            r = in(reg) value,
            options(nomem, preserves_flags, nostack),
        );
    }
}
