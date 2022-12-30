#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]

use core::ffi::c_void;
use core::ops::Range;

use gamecube_cpu::cache::{flush_data_cache_block, invalidate_instruction_cache_block};
use gamecube_cpu::registers::msr::{modify_msr, MachineState};
use gamecube_cpu::registers::time_base;
use gamecube_mmio::dvd_interface::DvdInterface;
use gamecube_mmio::processor_interface::{
    InterruptCause, InterruptMask, Interrupts, ProcessorInterface,
};
use gamecube_mmio::video_interface::{DisplayInterrupt, VideoInterface};
use gamecube_video_driver::framebuffer::Framebuffer;
use gamecube_video_driver::VideoDriver;
use mvbitfield::prelude::*;
use panic_abort as _;

mod os_globals;
mod paging;
mod text_console;

use crate::os_globals::OsGlobals;
use crate::text_console::{Font, TextConsole};

// Static data.
static FONT: &Font = Font::from_slice(include_bytes!("../../../build/console_font.dat"));

// Large buffers.
#[link_section = ".bss"]
static FRAMEBUFFER: Framebuffer = Framebuffer::zero();

// Interrupt handling.
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

unsafe fn install_interrupt_handlers() {
    install_interrupt_handler(
        0x80000200usize as _,
        &machine_check_exception_handler_start..&machine_check_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000300usize as _,
        &dsi_exception_handler_start..&dsi_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000400usize as _,
        &isi_exception_handler_start..&isi_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000500usize as _,
        &external_interrupt_exception_handler_start..&external_interrupt_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000600usize as _,
        &alignment_exception_handler_start..&alignment_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000700usize as _,
        &program_exception_handler_start..&program_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000800usize as _,
        &fp_unavailable_exception_handler_start..&fp_unavailable_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000900usize as _,
        &decrementer_exception_handler_start..&decrementer_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000c00usize as _,
        &system_call_exception_handler_start..&system_call_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000d00usize as _,
        &trace_exception_handler_start..&trace_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000e00usize as _,
        &fp_assist_exception_handler_start..&fp_assist_exception_handler_end,
    );
    install_interrupt_handler(
        0x80000f00usize as _,
        &performance_monitor_exception_handler_start..&performance_monitor_exception_handler_end,
    );
    install_interrupt_handler(
        0x80001300usize as _,
        &breakpoint_exception_handler_start..&breakpoint_exception_handler_end,
    );
    install_interrupt_handler(
        0x80001700usize as _,
        &thermal_management_exception_handler_start..&thermal_management_exception_handler_end,
    );
}

unsafe fn install_interrupt_handler(dst: *mut c_void, src_range: Range<*const c_void>) {
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

struct Devices {
    globals: OsGlobals<'static>,
    di: DvdInterface<'static>,
    pi: ProcessorInterface<'static>,
    vi: VideoInterface<'static>,
}

unsafe fn init() -> Devices {
    modify_msr(|msr| {
        msr.with_external_interrupts_enabled(false)
            .with_machine_check_enabled(false)
    });

    install_interrupt_handlers();

    modify_msr(|msr| {
        msr.with_external_interrupts_enabled(true)
            .with_machine_check_enabled(true)
    });

    // SAFETY: These must be the only calls in the program.
    Devices {
        globals: unsafe { OsGlobals::new_unchecked() },
        di: unsafe { DvdInterface::new_unchecked() },
        pi: unsafe { ProcessorInterface::new_unchecked() },
        vi: unsafe { VideoInterface::new_unchecked() },
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    let Devices {
        globals,
        mut pi,
        vi,
        ..
    } = unsafe { init() };

    let mut video = VideoDriver::new(vi);
    video.configure_for_ntsc_480p(FRAMEBUFFER.as_ptr().cast());

    // Acknowledge any pending PI interrupts and enable VI interrupts.
    pi.write_interrupt_cause(InterruptCause::zero().with_interrupts(Interrupts::all()));
    pi.write_interrupt_mask(
        InterruptMask::zero().with_interrupts(Interrupts::zero().with_video_interface(true)),
    );

    let mut console = TextConsole::new();
    let mut counter = 0;
    let mut last_time = 0;
    loop {
        if globals.read_vi_interrupt_fired() {
            globals.write_vi_interrupt_fired(false);
            let start_time = time_base();
            console.render(&FONT, &FRAMEBUFFER);
            let end_time = time_base();

            let elapsed_us = (2 * (end_time - start_time) / 81) as u32;
            console.print_str("\nText console render time: 0x");
            console.print_hex_u32(elapsed_us);
            console.print_str(" us");

            console.print_str("\nCounter: 0x");
            console.print_hex_u32(counter);
            counter = counter.wrapping_add(1);

            let elapsed_us = (2 * (end_time - last_time) / 81) as u32;
            console.print_str("\nFrame time: 0x");
            console.print_hex_u32(elapsed_us);
            console.print_str(" us");
            last_time = end_time;
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BsodArgs {
    vector: u32,
    srr0: u32,
    srr1: u32,
    dsisr: u32,
    dar: u32,
    gpr: [u32; 32],
    ibats: [Bat; 4],
    dbats: [Bat; 4],
    msr: MachineState,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Bat {
    u: u32,
    l: u32,
}

#[no_mangle]
pub extern "C" fn bsod(args: &BsodArgs) -> ! {
    // SAFETY: Interrupts have been disabled. We own the machine.
    let vi = unsafe { VideoInterface::new_unchecked() };

    let mut console = TextConsole::new();
    console.print_str(
        r#"
      ====================================================================
      = INCEPTION CYAN-TO-MAGENTA GRADIENT BORDERED SCREEN OF REFLECTION =
      ====================================================================

  An unexpected exception occurred. We didn't know what to do with it, so here
  are a lot of numbers.

  Exception vector 0x"#,
    );
    console.print_hex_u32(args.vector);

    console.print_str("\n  SRR0 0x");
    console.print_hex_u32(args.srr0);
    console.print_str("  SRR1 0x");
    console.print_hex_u32(args.srr1);
    console.print_str("  DSISR 0x");
    console.print_hex_u32(args.dsisr);
    console.print_str("  DAR 0x");
    console.print_hex_u32(args.dar);

    console.print_str("\n\n  MSR 0x");
    console.print_hex_u32(args.msr.as_u32());

    // Print GPRs.
    console.print_str("\n");
    for i in 0..32 {
        // Rearrange to count down each of the four columns.
        let gpr = i / 4 + (i % 4) * 8;
        console.print_str(if i & 3 == 0 { "\n  " } else { "  " });
        console.print_str("r");
        if gpr >= 10 {
            console.print_hex_digit((gpr / 10) as u8);
        }
        console.print_hex_digit((gpr % 10) as u8);
        if gpr < 10 {
            console.print_char(' ');
        }
        console.print_str(" 0x");
        console.print_hex_u32(args.gpr[gpr]);
    }

    // Print BATs.
    console.print_str("\n");
    for i in 0..4 {
        console.print_str("\n  IBAT");
        console.print_hex_digit(i as u8);
        console.print_str(" 0x");
        console.print_hex_u32(args.ibats[i].u);
        console.print_str("_");
        console.print_hex_u32(args.ibats[i].l);

        console.print_str("  DBAT");
        console.print_hex_digit(i as u8);
        console.print_str(" 0x");
        console.print_hex_u32(args.dbats[i].u);
        console.print_str("_");
        console.print_hex_u32(args.dbats[i].l);
    }

    console.render(&FONT, &FRAMEBUFFER);

    VideoDriver::new(vi).configure_for_ntsc_480i(FRAMEBUFFER.as_ptr().cast());

    loop {}
}
