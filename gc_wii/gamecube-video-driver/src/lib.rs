#![no_std]

use gamecube_mmio::video_interface::*;
use mvbitfield::prelude::*;

pub mod framebuffer;

pub struct VideoDriver {
    vi: VideoInterface,
}

impl VideoDriver {
    pub fn new(vi: VideoInterface) -> Self {
        Self { vi }
    }

    /// Configures the video interface for NTSC 480i output.
    ///
    /// The framebuffer must be 32-byte aligned and below physical memory address 0x01000000 (16
    /// MiB).
    ///
    /// # Timing
    ///
    /// The clock is set to 27 MHz, so ticks are 2 / 27 MHz = 74.(074) ns.
    ///
    /// ## Vertical
    ///
    /// There are 240 active lines.
    ///
    /// Line sequence in a field:
    /// - Equalization: 3x 3 lines
    /// - Pre-blanking: 12/12.5 lines
    /// - Active video: 240 lines
    /// - Post-blanking: 1.5/1 lines
    /// - Total: 262.5 lines
    ///
    /// Burst blanking:
    /// - Odd fields start 12 halflines into the field and end 520 halflines into the field
    /// - Even fields start 13 halflines into the field and end 519 halflines into the field
    ///
    /// ## Horizontal
    ///
    /// A half-line is 429 ticks and a full line is 858 ticks. There are 720 ticks of active
    /// display.
    ///
    /// ```text
    /// <===|------ Active display -------|===>
    ///     |=============================|-- Horizontal blanking
    ///     |     |=====|-----------------+-- Horizontal sync pulse
    ///     |     |     |     |=====|-----+-- Color burst
    ///
    /// <===|-----|_____|-----|^v^v^|-----|===>  Signal
    ///
    /// <===|-----+-----+-----+-----+-----+-- Halfline to hblank start:         373 ticks (27.6 us)
    ///           |=====|-----+-----+-----+-- HSync width:                       64 ticks ( 4.7 us)
    ///           |===========|-----+-----+-- HSync start to color burst start:  71 ticks ( 5.3 us)
    ///           |=================|-----+-- HSync start to color burst end:   105 ticks ( 7.8 us)
    ///           |=================|=====|-- HSync start to hblank end:        162 ticks (12.0 us)
    /// ```
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
        // Send an interrupt at the beginning of every vblank.
        // NOTE: I'm measuring alternating VI interrupt intervals of 16,651 us and 16,715 us. The
        // difference is almost precisely one NTSC scanline. So I think the second one needs to be
        // offset by a half-line.
        self.vi.write_display_interrupt_0(
            DisplayInterrupt::zero()
                .with_horizontal_position(U11::new_masked(641))
                .with_vertical_position(U11::new_masked(240)) // The last line of the first field.
                .with_interrupt_enable(true),
        );
        self.vi.write_display_interrupt_1(
            DisplayInterrupt::zero()
                .with_horizontal_position(U11::new_masked(641))
                .with_vertical_position(U11::new_masked(503)) // The last line of the second field.
                .with_interrupt_enable(true),
        );
        self.vi.write_display_interrupt_2(DisplayInterrupt::zero());
        self.vi.write_display_interrupt_3(DisplayInterrupt::zero());
        self.vi.write_horizontal_scaling(
            HorizontalScaling::zero()
                .with_step_size_u1_8(U9::new_masked(0x100))
                .with_enable(false)
                // 80 * 16 bytes = 1280 bytes, one line of a 640 pixel wide framebuffer. That's the
                // stride per half line, so every other line is displayed.
                .with_stride_per_half_line_in_16_byte_units(80)
                // 40 * 16 pixels = 640 pixels
                .with_framebuffer_width_in_16_pixel_units(U7::new_masked(40)),
        );
        self.vi
            .write_clock_select(ClockSelect::zero().with_clock(Clock::K27MHz));

        self.vi.write_display_configuration(
            DisplayConfiguration::zero()
                .with_enable(true)
                .with_interlace(Interlace::Interlaced)
                .with_format(Format::Ntsc),
        );
    }

    /// Configures the video interface for NTSC 480p output.
    ///
    /// The framebuffer must be 32-byte aligned and below physical memory address 0x01000000 (16
    /// MiB).
    ///
    /// # Timing
    ///
    /// The clock is set to 54 MHz, so ticks are 2 / 54 MHz = 37.(037) ns.
    ///
    /// ## Vertical
    ///
    /// There are 480 active lines.
    ///
    /// Line sequence in a field:
    /// - Equalization: 3x 6 lines
    /// - Pre-blanking: 24/24 lines
    /// - Active video: 480 lines
    /// - Post-blanking: 3/3 lines
    /// - Total: 525 lines
    ///
    /// Note that there are still two fields over a total of 1050 lines. But they produce
    /// indistinguishable signals.
    ///
    /// Burst blanking:
    /// - All fields start 24 halflines into the field and end 1038 halflines into the field
    ///
    /// ## Horizontal
    ///
    /// A half-line is 429 ticks and a full line is 858 ticks. There are 720 ticks of active
    /// display.
    ///
    /// ```text
    /// <===|------ Active display -------|===>
    ///     |=============================|-- Horizontal blanking
    ///     |     |=====|-----------------+-- Horizontal sync pulse
    ///     |     |     |     |=====|-----+-- Color burst
    ///
    /// <===|-----|_____|-----|^v^v^|-----|===>  Signal
    ///
    /// <===|-----+-----+-----+-----+-----+-- Halfline to hblank start:         373 ticks (13.8 us)
    ///           |=====|-----+-----+-----+-- HSync width:                       64 ticks ( 2.4 us)
    ///           |===========|-----+-----+-- HSync start to color burst start:  71 ticks ( 2.6 us)
    ///           |=================|-----+-- HSync start to color burst end:   105 ticks ( 3.9 us)
    ///           |=======================|-- HSync start to hblank end:        162 ticks ( 6.0 us)
    /// ```
    pub fn configure_for_ntsc_480p(&mut self, framebuffer: *const ()) {
        self.vi
            .write_display_configuration(DisplayConfiguration::zero().with_reset(true));

        self.vi.write_vertical_timing_a(
            VerticalTimingA::zero()
                .with_equalization_pulse_half_lines(U4::new_masked(12))
                .with_active_video_lines(U10::new_masked(480)),
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
                .with_pre_blanking_half_lines(U10::new_masked(36))
                .with_post_blanking_half_lines(U10::new_masked(18)),
        );
        self.vi.write_vertical_timing_b_even_field(
            VerticalTimingB::zero()
                .with_pre_blanking_half_lines(U10::new_masked(36))
                .with_post_blanking_half_lines(U10::new_masked(18)),
        );
        self.vi.write_burst_blanking_odd_field(
            BurstBlankingOddField::zero()
                .with_field_1_start_to_burst_blanking_start_half_lines(U5::new_masked(24))
                .with_field_1_start_to_burst_blanking_end_half_lines(U11::new_masked(1038))
                .with_field_3_start_to_burst_blanking_start_half_lines(U5::new_masked(24))
                .with_field_3_start_to_burst_blanking_end_half_lines(U11::new_masked(1038)),
        );
        self.vi.write_burst_blanking_even_field(
            BurstBlankingEvenField::zero()
                .with_field_2_start_to_burst_blanking_start_half_lines(U5::new_masked(24))
                .with_field_2_start_to_burst_blanking_end_half_lines(U11::new_masked(1038))
                .with_field_4_start_to_burst_blanking_start_half_lines(U5::new_masked(24))
                .with_field_4_start_to_burst_blanking_end_half_lines(U11::new_masked(1038)),
        );
        self.vi.write_top_left_field_base(
            FieldBase::zero()
                .with_addresss(U24::new_masked(framebuffer as u32))
                .with_shift_address_left_five(false),
        );
        self.vi.write_bottom_left_field_base(
            FieldBase::zero()
                .with_addresss(U24::new_masked(framebuffer as u32))
                .with_shift_address_left_five(false),
        );
        // Send an interrupt at the beginning of every vblank.
        self.vi.write_display_interrupt_0(
            DisplayInterrupt::zero()
                .with_horizontal_position(U11::new_masked(641))
                .with_vertical_position(U11::new_masked(480)) // The last line of the first field.
                .with_interrupt_enable(true),
        );
        self.vi.write_display_interrupt_1(
            DisplayInterrupt::zero()
                .with_horizontal_position(U11::new_masked(641))
                .with_vertical_position(U11::new_masked(1005)) // The last line of the second field.
                .with_interrupt_enable(true),
        );
        self.vi.write_display_interrupt_2(DisplayInterrupt::zero());
        self.vi.write_display_interrupt_3(DisplayInterrupt::zero());
        self.vi.write_horizontal_scaling(
            HorizontalScaling::zero()
                .with_step_size_u1_8(U9::new_masked(0x100))
                .with_enable(false)
                // 40 * 16 bytes = 640 bytes, one half-line of a 640 pixel wide framebuffer.
                .with_stride_per_half_line_in_16_byte_units(40)
                // 40 * 16 pixels = 640 pixels
                .with_framebuffer_width_in_16_pixel_units(U7::new_masked(40)),
        );
        self.vi
            .write_clock_select(ClockSelect::zero().with_clock(Clock::K54MHz));

        self.vi.write_display_configuration(
            DisplayConfiguration::zero()
                .with_enable(true)
                .with_interlace(Interlace::NonInterlaced)
                .with_format(Format::Ntsc),
        );
    }
}
