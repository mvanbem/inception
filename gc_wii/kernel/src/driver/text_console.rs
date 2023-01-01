use gamecube_mmio::video_interface::VideoInterface;
use gamecube_video_driver::framebuffer::Framebuffer;
use gamecube_video_driver::VideoDriver;

use crate::driver::driver_state::DriverState;
use crate::system_call;
use crate::text_console::TextConsole;
use crate::thread::{create_thread, VideoInterfaceInterrupt, WaitingFor, USER_MACHINE_STATE};

#[link_section = ".bss"]
pub static FRAMEBUFFER: Framebuffer = Framebuffer::zero();

#[link_section = ".bss"]
static STATE: DriverState<TextConsole> = DriverState::uninit();

pub fn init() {
    STATE.init_with(TextConsole::new);
    create_thread(render_thread, USER_MACHINE_STATE, None);
}

pub fn print(s: &str) {
    STATE.with_state(|text_console| {
        use core::fmt::Write;

        write!(text_console, "{}", s).unwrap();
    });
}

fn render_if_changed() {
    STATE.with_state(|text_console| {
        if text_console.modified() {
            text_console.render(&FRAMEBUFFER);
        }
    });
}

extern "C" fn render_thread() -> ! {
    VideoDriver::new(VideoInterface::new()).configure_for_ntsc_480p(FRAMEBUFFER.as_ptr().cast());

    loop {
        render_if_changed();
        system_call::wait_for(
            WaitingFor::zero().with_video_interface(
                VideoInterfaceInterrupt::zero()
                    .with_display_interrupt_0(true)
                    .with_display_interrupt_1(true),
            ),
        )
    }
}
