use core::ffi::c_void;
use core::ops::Range;

use gamecube_cpu::cache::{flush_data_cache_block, invalidate_instruction_cache_block};

extern "C" {
    static machine_check_exception_handler_start: c_void;
    static machine_check_exception_handler_end: c_void;
    static dsi_exception_handler_start: c_void;
    static dsi_exception_handler_end: c_void;
    static isi_exception_handler_start: c_void;
    static isi_exception_handler_end: c_void;
    static external_interrupt_exception_handler_start: c_void;
    static external_interrupt_exception_handler_end: c_void;
    static alignment_exception_handler_start: c_void;
    static alignment_exception_handler_end: c_void;
    static program_exception_handler_start: c_void;
    static program_exception_handler_end: c_void;
    static fp_unavailable_exception_handler_start: c_void;
    static fp_unavailable_exception_handler_end: c_void;
    static decrementer_exception_handler_start: c_void;
    static decrementer_exception_handler_end: c_void;
    static system_call_exception_handler_start: c_void;
    static system_call_exception_handler_end: c_void;
    static trace_exception_handler_start: c_void;
    static trace_exception_handler_end: c_void;
    static fp_assist_exception_handler_start: c_void;
    static fp_assist_exception_handler_end: c_void;
    static performance_monitor_exception_handler_start: c_void;
    static performance_monitor_exception_handler_end: c_void;
    static breakpoint_exception_handler_start: c_void;
    static breakpoint_exception_handler_end: c_void;
    static thermal_management_exception_handler_start: c_void;
    static thermal_management_exception_handler_end: c_void;
}

pub unsafe fn install_exception_handlers() {
    install_exception_handler(
        0x80000200usize as _,
        &machine_check_exception_handler_start..&machine_check_exception_handler_end,
    );
    install_exception_handler(
        0x80000300usize as _,
        &dsi_exception_handler_start..&dsi_exception_handler_end,
    );
    install_exception_handler(
        0x80000400usize as _,
        &isi_exception_handler_start..&isi_exception_handler_end,
    );
    install_exception_handler(
        0x80000500usize as _,
        &external_interrupt_exception_handler_start..&external_interrupt_exception_handler_end,
    );
    install_exception_handler(
        0x80000600usize as _,
        &alignment_exception_handler_start..&alignment_exception_handler_end,
    );
    install_exception_handler(
        0x80000700usize as _,
        &program_exception_handler_start..&program_exception_handler_end,
    );
    install_exception_handler(
        0x80000800usize as _,
        &fp_unavailable_exception_handler_start..&fp_unavailable_exception_handler_end,
    );
    install_exception_handler(
        0x80000900usize as _,
        &decrementer_exception_handler_start..&decrementer_exception_handler_end,
    );
    install_exception_handler(
        0x80000c00usize as _,
        &system_call_exception_handler_start..&system_call_exception_handler_end,
    );
    install_exception_handler(
        0x80000d00usize as _,
        &trace_exception_handler_start..&trace_exception_handler_end,
    );
    install_exception_handler(
        0x80000e00usize as _,
        &fp_assist_exception_handler_start..&fp_assist_exception_handler_end,
    );
    install_exception_handler(
        0x80000f00usize as _,
        &performance_monitor_exception_handler_start..&performance_monitor_exception_handler_end,
    );
    install_exception_handler(
        0x80001300usize as _,
        &breakpoint_exception_handler_start..&breakpoint_exception_handler_end,
    );
    install_exception_handler(
        0x80001700usize as _,
        &thermal_management_exception_handler_start..&thermal_management_exception_handler_end,
    );
}

unsafe fn install_exception_handler(dst: *mut c_void, src_range: Range<*const c_void>) {
    // Copy 32-byte blocks, flushing as we go.
    let mut src: *const u32 = src_range.start.cast();
    let mut dst: *mut u32 = dst.cast();
    while src != src_range.end.cast() {
        let dst_block_start = dst;
        for _ in 0..8 {
            *dst = *src;
            src = src.offset(1);
            dst = dst.offset(1);
        }
        flush_data_cache_block(dst_block_start as _);
        invalidate_instruction_cache_block(dst_block_start as _);
    }
}
