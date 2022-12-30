use core::fmt::Write;

use gamecube_cpu::registers::msr::MachineState;
use gamecube_mmio::video_interface::VideoInterface;
use gamecube_video_driver::VideoDriver;

use crate::text_console::TextConsole;
use crate::FRAMEBUFFER;

#[derive(Clone, Copy)]
#[repr(C)]
struct BsodArgs {
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
struct Bat {
    u: u32,
    l: u32,
}

#[no_mangle]
extern "C" fn bsod(args: &BsodArgs) -> ! {
    // SAFETY: Interrupts have been disabled. We own the machine.
    let vi = unsafe { VideoInterface::new_unchecked() };

    let mut console = TextConsole::new();
    writeln!(
        &mut console,
        r#"
      ====================================================================
      = INCEPTION CYAN-TO-MAGENTA GRADIENT BORDERED SCREEN OF REFLECTION =
      ====================================================================

  An unexpected exception occurred. We didn't know what to do with it, so here
  are a lot of numbers.

  Exception vector 0x{vector:08x}
  SRR0 0x{srr0:08x}  SRR1 0x{srr1:08x}  DSISR 0x{dsisr:08x}  DAR 0x{dar:08x}

  MSR 0x{msr:08x}

"#,
        vector = args.vector,
        srr0 = args.srr0,
        srr1 = args.srr1,
        dsisr = args.dsisr,
        dar = args.dar,
        msr = args.msr.as_u32(),
    )
    .unwrap();

    // Print GPRs.
    for i in 0..32 {
        // Rearrange to count down each of the four columns.
        let gpr = i / 4 + (i % 4) * 8;
        write!(
            &mut console,
            "r{gpr:2} 0x{value:08x}{suffix}",
            value = args.gpr[gpr],
            suffix = if i & 3 == 3 { "\n" } else { "  " },
        )
        .unwrap();
    }

    // Print BATs.
    writeln!(&mut console).unwrap();
    for i in 0..4 {
        writeln!(
            &mut console,
            "  IBAT{i} 0x{iu:08x}_{il:08x}  DBAT{i} 0x{du:08x}_{dl:08x}",
            iu = args.ibats[i].u,
            il = args.ibats[i].l,
            du = args.dbats[i].u,
            dl = args.dbats[i].l,
        )
        .unwrap();
    }

    console.render(&FRAMEBUFFER);

    VideoDriver::new(vi).configure_for_ntsc_480i(FRAMEBUFFER.as_ptr().cast());

    loop {}
}
