#![no_std]
#![feature(core_intrinsics, start)]

use ogc::prelude::*;

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    let mut video = Video::init();
    Video::configure(RenderConfig {
        ..video.render_config
    });
    Video::set_next_framebuffer(video.framebuffer);
    Video::set_black(false);
    Video::flush();
    Video::wait_vsync();

    video.clear_framebuffer(
        RenderConfig {
            ..video.render_config
        },
        0x80ff8000,
    );

    loop {
        Video::wait_vsync();
    }
}
