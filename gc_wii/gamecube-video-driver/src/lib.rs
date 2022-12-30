//! # Display timing
//!
//! Everything is measured in ticks, which are two clocks in duration. The clock is configurable at
//! 27 or 54 MHz. NTSC 480i uses 27 MHz, so ticks are 2 / 27 MHz = 74.(074) ns.
//!
//! ## Vertical
//!
//! There are 240 active lines.
//!
//! Burst blanking:
//! - Odd fields start 12 halflines into the field and end 520 halfines into the field
//! - Even fields start 13 halflines into the field and end 519 halflines into the field
//! - What does this mean? Does this refer to the color burst?
//!
//! ## Horizontal
//!
//! A half-line is 429 ticks and a full line 858 ticks. There are 720 ticks of active display.
//!
//! - 71 ticks from hsync start to color burst start
//! - 105 ticks from hsync start to color burst end
//!     - So the color burst is 34 ticks or 2.52 us
//! - hsync is 64 ticks
//! - 162 ticks from hsync start to hblank end
//! - Horizontal blanking begins 373 ticks after the middle of the line.
//!
//! |-- one line, 858 ticks --------------------------------------------------------------|
//! |-- one half-line, 429 ticks --------------|-- one half-line, 429 ticks --------------|
//! |                                          |                                          |
//! |___|======================================|================================|_________|
//!     |                                                                       |
//!     |-- active display, 720 ticks                                           |
//! ..--|                                        horizontal blanking, ??? ticks |--------..
//!                                            |-- HBS, 373 ticks --------------|

#![no_std]

use gamecube_mmio::video_interface::*;
use mvbitfield::prelude::*;

pub mod framebuffer;

pub struct VideoDriver {
    vi: VideoInterface<'static>,
}

impl VideoDriver {
    pub fn new(vi: VideoInterface<'static>) -> Self {
        Self { vi }
    }

    pub fn registers_mut(&mut self) -> VideoInterface {
        self.vi.reborrow()
    }

    /// framebuffer must be 32-byte aligned and below physical memory address 0x01000000 (16 MiB).
    pub fn configure_for_ntsc_480i(&mut self, framebuffer: *const ()) {
        self.vi
            .write_display_configuration(DisplayConfiguration::zero().with_reset(true));

        self.vi.write_vertical_timing_a(
            VerticalTimingA::zero()
                .with_equalization_pulse_half_lines(U4::new_masked(6))
                .with_active_video_lines(U10::new_masked(240)),
        );
        self.vi.write_horizontal_timing_a(
            HorizontalTimingA::zero()
                .with_halfline_width(U9::new_masked(429))
                .with_hsync_start_to_color_burst_end(U7::new_masked(105))
                .with_hsync_start_to_color_burst_start(U7::new_masked(71)),
        );
        self.vi.write_horizontal_timing_b(
            HorizontalTimingB::zero()
                .with_hsync_width(U7::new_masked(64))
                .with_hsync_start_to_hblank_end(U10::new_masked(162))
                .with_half_line_to_hblank_start(U10::new_masked(373)),
        );
        self.vi.write_vertical_timing_b_odd_field(
            VerticalTimingB::zero()
                .with_pre_blanking_half_lines(U10::new_masked(24))
                .with_post_blanking_half_lines(U10::new_masked(3)),
        );
        self.vi.write_vertical_timing_b_even_field(
            VerticalTimingB::zero()
                .with_pre_blanking_half_lines(U10::new_masked(25))
                .with_post_blanking_half_lines(U10::new_masked(2)),
        );
        self.vi.write_burst_blanking_odd_field(
            BurstBlankingOddField::zero()
                .with_field_1_start_to_burst_blanking_start_half_lines(U5::new_masked(12))
                .with_field_1_start_to_burst_blanking_end_half_lines(U11::new_masked(520))
                .with_field_3_start_to_burst_blanking_start_half_lines(U5::new_masked(12))
                .with_field_3_start_to_burst_blanking_end_half_lines(U11::new_masked(520)),
        );
        self.vi.write_burst_blanking_even_field(
            BurstBlankingEvenField::zero()
                .with_field_2_start_to_burst_blanking_start_half_lines(U5::new_masked(13))
                .with_field_2_start_to_burst_blanking_end_half_lines(U11::new_masked(519))
                .with_field_4_start_to_burst_blanking_start_half_lines(U5::new_masked(13))
                .with_field_4_start_to_burst_blanking_end_half_lines(U11::new_masked(519)),
        );
        self.vi.write_top_left_field_base(
            FieldBase::zero()
                .with_addresss(U24::new_masked(framebuffer as u32))
                .with_shift_address_left_five(false),
        );
        self.vi.write_bottom_left_field_base(
            FieldBase::zero()
                // 1280 is the byte stride of a 640 pixel wide framebuffer. The bottom field
                // starts on the second line.
                .with_addresss(U24::new_masked(framebuffer as u32 + 1280))
                .with_shift_address_left_five(false),
        );
        self.vi.write_horizontal_scaling(
            HorizontalScaling::zero()
                .with_step_size_u1_8(U9::new_masked(0x100))
                .with_enable(false)
                // 80 * 16 = 1280 bytes, one line of a 640 pixel wide framebuffer. That's the
                // stride per half line, so every other line is displayed.
                .with_stride_per_half_line_in_16_byte_units(80)
                // 40 * 16 = 640 pixels
                .with_framebuffer_width_in_16_pixel_units(U7::new_masked(40)),
        );
        self.vi
            .write_clock_select(ClockSelect::zero().with_clock(Clock::_27MHz));

        self.vi.write_display_configuration(
            DisplayConfiguration::zero()
                .with_enable(true)
                .with_interlace(Interlace::Interlaced)
                .with_format(Format::Ntsc),
        );
    }
}
