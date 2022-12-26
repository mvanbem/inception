#![no_main]
#![no_std]

use gamecube_cpu::interrupts::disable_interrupts;
use gamecube_video_driver::framebuffer::Framebuffer;
use gamecube_video_driver::VideoInterface;
use panic_abort as _;

mod paging;
mod test_pattern;

use crate::test_pattern::initialize_framebuffer;

#[link_section = ".bss"]
static FRAMEBUFFER: Framebuffer = Framebuffer::zero();

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe { disable_interrupts() };

    // SAFETY: This must be the only call in the program.
    let mut vi = unsafe { VideoInterface::new_unchecked() };

    initialize_framebuffer(&FRAMEBUFFER);
    vi.configure_for_ntsc_480i(FRAMEBUFFER.as_ptr().cast());

    loop {}
}
