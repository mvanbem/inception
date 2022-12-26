#![feature(asm_experimental_arch)]
#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]

use core::arch::asm;

use gamecube_cpu::interrupts::disable_interrupts;
use gamecube_video_driver::VideoInterface;
use panic_abort as _;

mod paging;

unsafe fn initialize_framebuffer(framebuffer: *mut ()) {
    // Address of a single pixel, which consists of a Y luma byte and either a U or V chroma byte.
    // `s` is in 0..640; `t` is in 0..480.
    let pixel_addr = |s: usize, t: usize| framebuffer.cast::<u16>().offset((640 * t + s) as isize);

    // Address of a horizontal pixel pair.
    // `s` is in 0..320; `t` is in 0..480.
    let pixel_pair_addr =
        |s: usize, t: usize| framebuffer.cast::<u32>().offset((320 * t + s) as isize);

    // Clear to gray.
    let mut ptr: *mut u32 = framebuffer.cast();
    for _ in 0..320 * 480 {
        *ptr = 0x80808080;
        ptr = ptr.offset(1);
    }

    // Fill a chroma rectangle in the middle.
    for t in 0..256 {
        for s in 0..128 {
            let (y, u, v) = (0x80, (2 * s) as u8, (255 - t) as u8);
            let word = ((y as u32) << 24) | ((u as u32) << 16) | ((y as u32) << 8) | v as u32;
            let ptr = pixel_pair_addr(s + 96, t + 112);
            *ptr = word;
        }
    }

    // Draw binary rulers around the middle.
    let draw_horizontal_ruler_row = |t, mask| {
        for s in 0..640 {
            *pixel_addr(s, t) = if (s & mask) == 0 { 0x0080 } else { 0xff80 };
        }
    };
    let draw_horizontal_ruler = |t_start: usize, t_delta: isize, width: usize| {
        let mut t = t_start;
        let mut mask = 1 << 9;
        for _ in 0..=9 {
            for _ in 0..width {
                draw_horizontal_ruler_row(t, mask);
                t = t.wrapping_add_signed(t_delta);
            }
            mask >>= 1;
        }
    };
    draw_horizontal_ruler(111, -1, 3);
    draw_horizontal_ruler(368, 1, 3);

    let draw_vertical_ruler_column = |s, mask| {
        for t in 0..480 {
            let ptr = pixel_addr(s, t);
            *ptr = if *ptr == 0x8080 {
                if (t & mask) == 0 {
                    0x0080
                } else {
                    0xff80
                }
            } else {
                0x8080
            }
        }
    };
    let draw_vertical_ruler = |s_start: usize, s_delta: isize, width: usize| {
        let mut s = s_start;
        let mut mask = 1 << 8;
        for _ in 0..=8 {
            for _ in 0..width {
                draw_vertical_ruler_column(s, mask);
                s = s.wrapping_add_signed(s_delta);
            }
            mask >>= 1;
        }
    };
    draw_vertical_ruler(191, -1, 3);
    draw_vertical_ruler(448, 1, 3);

    data_cache_flush_aligned(framebuffer.cast(), 640 * 480 * 2 / 32);
}

/// Flushes a sequence of 32-byte cache blocks starting with the given aligned address.
#[inline(never)]
unsafe fn data_cache_flush_aligned(mut ptr: *const (), block_count: usize) {
    for _ in 0..block_count {
        asm!(
            "dcbf 0,{r}",
            r = in(reg) ptr,
            options(preserves_flags, nostack),
        );
        ptr = ptr.byte_offset(32);
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe { disable_interrupts() };

    // SAFETY: This must be the only call in the program.
    let mut vi = unsafe { VideoInterface::new_unchecked() };

    // This is a kind of arbitrary location halfway through the XFB fine range. We need to be in the
    // fine range to be able to address the first two lines of a tightly packed interlaced buffer.
    let framebuffer: *mut () = 0x80040000usize as _;

    unsafe { initialize_framebuffer(framebuffer) }
    vi.configure_for_ntsc_480i(framebuffer as _);

    loop {}
}
