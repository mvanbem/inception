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

use core::marker::PhantomData;

use mvbitfield::prelude::*;

use crate::registers::*;

mod registers {
    #![allow(dead_code)]

    use core::mem::{size_of, transmute};
    use core::ptr;

    use mvbitfield::prelude::*;

    /// Base address 0xcc002000
    #[repr(C)]
    pub struct VI {
        vertical_timing_a: VerticalTimingA,
        display_configuration: DisplayConfiguration,
        horizontal_timing_a: HorizontalTimingA,
        horizontal_timing_b: HorizontalTimingB,
        vertical_timing_b_odd_field: VerticalTimingB,
        vertical_timing_b_even_field: VerticalTimingB,
        burst_blanking_odd_field: BurstBlankingOddField,
        burst_blanking_even_field: BurstBlankingEvenField,
        top_left_field_base: FieldBase,
        top_right_field_base: FieldBase,
        bottom_left_field_base: FieldBase,
        bottom_right_field_base: FieldBase,
        vertical_position: u16,
        horizontal_position: u16,
        display_interrupt: [DisplayInterrupt; 4],
        display_latch: [DisplayLatch; 2],
        horizontal_scaling: HorizontalScaling,
        filter_coefficient_table: [u32; 8],
        clock_select: ClockSelect,
        _padding_todo_there_are_more_registers: u16,
    }

    const _: () = assert!(size_of::<VI>() == 0x70);

    impl VI {
        pub const PTR: *mut Self = 0xcc002000usize as _;

        pub unsafe fn write_vertical_timing_a(value: VerticalTimingA) {
            ptr::write_volatile(&mut (*Self::PTR).vertical_timing_a, value);
        }

        pub unsafe fn write_display_configuration(value: DisplayConfiguration) {
            ptr::write_volatile(&mut (*Self::PTR).display_configuration, value);
        }

        pub unsafe fn write_horizontal_timing_a(value: HorizontalTimingA) {
            ptr::write_volatile(&mut (*Self::PTR).horizontal_timing_a, value);
        }

        pub unsafe fn write_horizontal_timing_b(value: HorizontalTimingB) {
            ptr::write_volatile(&mut (*Self::PTR).horizontal_timing_b, value);
        }

        pub unsafe fn write_vertical_timing_b_odd_field(value: VerticalTimingB) {
            ptr::write_volatile(&mut (*Self::PTR).vertical_timing_b_odd_field, value);
        }

        pub unsafe fn write_vertical_timing_b_even_field(value: VerticalTimingB) {
            ptr::write_volatile(&mut (*Self::PTR).vertical_timing_b_even_field, value);
        }

        pub unsafe fn write_burst_blanking_odd_field(value: BurstBlankingOddField) {
            ptr::write_volatile(&mut (*Self::PTR).burst_blanking_odd_field, value);
        }

        pub unsafe fn write_burst_blanking_even_field(value: BurstBlankingEvenField) {
            ptr::write_volatile(&mut (*Self::PTR).burst_blanking_even_field, value);
        }

        pub unsafe fn write_top_left_field_base(value: FieldBase) {
            ptr::write_volatile(&mut (*Self::PTR).top_left_field_base, value);
        }

        pub unsafe fn write_bottom_left_field_base(value: FieldBase) {
            ptr::write_volatile(&mut (*Self::PTR).bottom_left_field_base, value);
        }

        pub unsafe fn write_display_interrupt(value: [DisplayInterrupt; 4]) {
            ptr::write_volatile(&mut (*Self::PTR).display_interrupt[0], value[0]);
            ptr::write_volatile(&mut (*Self::PTR).display_interrupt[1], value[1]);
            ptr::write_volatile(&mut (*Self::PTR).display_interrupt[2], value[2]);
            ptr::write_volatile(&mut (*Self::PTR).display_interrupt[3], value[3]);
        }

        pub unsafe fn write_display_latch(value: [DisplayLatch; 2]) {
            ptr::write_volatile(&mut (*Self::PTR).display_latch[0], value[0]);
            ptr::write_volatile(&mut (*Self::PTR).display_latch[1], value[1]);
        }

        pub unsafe fn write_horizontal_scaling(value: HorizontalScaling) {
            ptr::write_volatile(&mut (*Self::PTR).horizontal_scaling, value);
        }

        pub unsafe fn write_clock_select(value: ClockSelect) {
            ptr::write_volatile(&mut (*Self::PTR).clock_select, value);
        }
    }

    mvbitfield! {
        pub struct VerticalTimingA: u16 {
            pub equalization_pulse_half_lines: 4,
            pub active_video_lines: 10,
        }
    }

    mvbitfield! {
        pub struct DisplayConfiguration: u16 {
            pub enable: 1 as bool,
            pub reset: 1 as bool,
            pub interlace: 1 as Interlace,
            pub three_d_display: 1 as bool,
            pub latch_0_enable: 2 as Latch,
            pub latch_1_enable: 2 as Latch,
            pub format: 2 as Format,
        }
    }

    #[repr(u8)]
    pub enum Interlace {
        Interlaced = 0,
        NonInterlaced = 1,
    }

    impl Interlace {
        pub const fn from_u1(value: U1) -> Self {
            // SAFETY: Interlace and U1 have the same layout and valid bit patterns.
            unsafe { transmute(value) }
        }

        pub const fn as_u1(self) -> U1 {
            // SAFETY: Interlace and U1 have the same layout and valid bit patterns.
            unsafe { transmute(self) }
        }
    }

    #[repr(u8)]
    pub enum Latch {
        Off = 0,
        OnForOneField = 1,
        OnForTwoFields = 2,
        AlwaysOn = 3,
    }

    impl Latch {
        pub const fn from_u2(value: U2) -> Self {
            // SAFETY: Latch and U2 have the same layout and valid bit patterns.
            unsafe { transmute(value) }
        }

        pub const fn as_u2(self) -> U2 {
            // SAFETY: Latch and U2 have the same layout and valid bit patterns.
            unsafe { transmute(self) }
        }
    }

    #[repr(u8)]
    pub enum Format {
        Ntsc = 0,
        Pal = 1,
        Mpal = 2,
        Debug = 3,
    }

    impl Format {
        pub const fn from_u2(value: U2) -> Self {
            // SAFETY: Format and U2 have the same layout and valid bit patterns.
            unsafe { transmute(value) }
        }

        pub const fn as_u2(self) -> U2 {
            // SAFETY: Format and U2 have the same layout and valid bit patterns.
            unsafe { transmute(self) }
        }
    }

    mvbitfield! {
        pub struct HorizontalTimingA: u32 {
            pub halfline_width: 9,
            _reserved: 7,
            pub hsync_start_to_color_burst_end: 7,
            _reserved: 1,
            pub hsync_start_to_color_burst_start: 7,
        }
    }

    mvbitfield! {
        pub struct HorizontalTimingB: u32 {
            pub hsync_width: 7,
            pub hsync_start_to_hblank_end: 10,
            pub half_line_to_hblank_start: 10,
        }
    }

    mvbitfield! {
        pub struct VerticalTimingB: u32 {
            pub pre_blanking_half_lines: 10,
            _reserved: 6,
            pub post_blanking_half_lines: 10,
        }
    }

    mvbitfield! {
        pub struct BurstBlankingOddField: u32 {
            pub field_1_start_to_burst_blanking_start_half_lines: 5,
            pub field_1_start_to_burst_blanking_end_half_lines: 11,
            pub field_3_start_to_burst_blanking_start_half_lines: 5,
            pub field_3_start_to_burst_blanking_end_half_lines: 11,
        }
    }

    mvbitfield! {
        pub struct BurstBlankingEvenField: u32 {
            pub field_2_start_to_burst_blanking_start_half_lines: 5,
            pub field_2_start_to_burst_blanking_end_half_lines: 11,
            pub field_4_start_to_burst_blanking_start_half_lines: 5,
            pub field_4_start_to_burst_blanking_end_half_lines: 11,
        }
    }

    mvbitfield! {
        pub struct FieldBase: u32 {
            // Must be 9-bit aligned.
            pub addresss: 24,
            pub horizontal_offset: 4,
            pub shift_address_left_five: 1 as bool,
        }
    }

    mvbitfield! {
        pub struct DisplayInterrupt: u32 {
            pub horizontal_position: 10,
            _reserved: 6,
            pub vertical_position: 10,
            _reserved: 2,
            pub interrupt_enable: 1 as bool,
            _reserved: 2,
            pub interrupt_status: 1 as bool,
        }
    }

    mvbitfield! {
        pub struct DisplayLatch: u32 {
            pub horizontal_count: 11,
            _reserved: 5,
            pub vertical_count: 11,
            _reserved: 4,
            pub trigger: 1 as bool,
        }
    }

    mvbitfield! {
        pub struct HorizontalScaling: u32 {
            pub step_size_u1_8: 9,
            _reserved: 3,
            pub enable: 1 as bool,
            _reserved: 3,
            pub stride_per_half_line_in_16_byte_units: 8,
            pub framebuffer_width_in_16_pixel_units: 7,
        }
    }

    mvbitfield! {
        pub struct ClockSelect: u16 {
            pub clock: 1 as Clock,
        }
    }

    #[repr(u8)]
    pub enum Clock {
        _27MHz = 0,
        _54MHz = 1,
    }

    impl Clock {
        pub const fn from_u1(value: U1) -> Self {
            // SAFETY: Clock and U1 have the same layout and valid bit patterns.
            unsafe { transmute(value) }
        }

        pub const fn as_u1(self) -> U1 {
            // SAFETY: Clock and U1 have the same layout and valid bit patterns.
            unsafe { transmute(self) }
        }
    }
}

pub struct VideoInterface {
    _phantom_non_send: PhantomData<*const ()>,
}

impl VideoInterface {
    /// # Safety
    ///
    /// This can only be called once.
    #[inline(always)]
    pub unsafe fn new_unchecked() -> Self {
        VideoInterface {
            _phantom_non_send: PhantomData,
        }
    }

    /// framebuffer must be 16-byte aligned and below physical memory address 0x000080000 (512 KiB).
    pub fn configure_for_ntsc_480i(&mut self, framebuffer: *const ()) {
        // SAFETY: The VI does not perform writes and cannot violate memory safety, even if
        // misconfigured. Holding an exclusive reference to a VideoInterface ensures we have
        // exclusive access to the device.
        unsafe {
            VI::write_display_configuration(DisplayConfiguration::zero().with_reset(true));

            VI::write_vertical_timing_a(
                VerticalTimingA::zero()
                    .with_equalization_pulse_half_lines(U4::new_masked(6))
                    .with_active_video_lines(U10::new_masked(240)),
            );
            VI::write_horizontal_timing_a(
                HorizontalTimingA::zero()
                    .with_halfline_width(U9::new_masked(429))
                    .with_hsync_start_to_color_burst_end(U7::new_masked(105))
                    .with_hsync_start_to_color_burst_start(U7::new_masked(71)),
            );
            VI::write_horizontal_timing_b(
                HorizontalTimingB::zero()
                    .with_hsync_width(U7::new_masked(64))
                    .with_hsync_start_to_hblank_end(U10::new_masked(162))
                    .with_half_line_to_hblank_start(U10::new_masked(373)),
            );
            VI::write_vertical_timing_b_odd_field(
                VerticalTimingB::zero()
                    .with_pre_blanking_half_lines(U10::new_masked(24))
                    .with_post_blanking_half_lines(U10::new_masked(3)),
            );
            VI::write_vertical_timing_b_even_field(
                VerticalTimingB::zero()
                    .with_pre_blanking_half_lines(U10::new_masked(25))
                    .with_post_blanking_half_lines(U10::new_masked(2)),
            );
            VI::write_burst_blanking_odd_field(
                BurstBlankingOddField::zero()
                    .with_field_1_start_to_burst_blanking_start_half_lines(U5::new_masked(12))
                    .with_field_1_start_to_burst_blanking_end_half_lines(U11::new_masked(520))
                    .with_field_3_start_to_burst_blanking_start_half_lines(U5::new_masked(12))
                    .with_field_3_start_to_burst_blanking_end_half_lines(U11::new_masked(520)),
            );
            VI::write_burst_blanking_even_field(
                BurstBlankingEvenField::zero()
                    .with_field_2_start_to_burst_blanking_start_half_lines(U5::new_masked(13))
                    .with_field_2_start_to_burst_blanking_end_half_lines(U11::new_masked(519))
                    .with_field_4_start_to_burst_blanking_start_half_lines(U5::new_masked(13))
                    .with_field_4_start_to_burst_blanking_end_half_lines(U11::new_masked(519)),
            );
            VI::write_top_left_field_base(
                FieldBase::zero()
                    .with_addresss(U24::new_masked(framebuffer as u32))
                    .with_shift_address_left_five(false),
            );
            VI::write_bottom_left_field_base(
                FieldBase::zero()
                    // 1280 is the byte stride of a 640 pixel wide framebuffer. The bottom field
                    // starts on the second line.
                    .with_addresss(U24::new_masked(framebuffer as u32 + 1280))
                    .with_shift_address_left_five(false),
            );
            VI::write_horizontal_scaling(
                HorizontalScaling::zero()
                    .with_step_size_u1_8(U9::new_masked(0x100))
                    .with_enable(false)
                    // 80 * 16 = 1280 bytes, one line of a 640 pixel wide framebuffer. That's the
                    // stride per half line, so every other line is displayed.
                    .with_stride_per_half_line_in_16_byte_units(80)
                    // 40 * 16 = 640 pixels
                    .with_framebuffer_width_in_16_pixel_units(U7::new_masked(40)),
            );
            VI::write_clock_select(ClockSelect::zero().with_clock(Clock::_27MHz));

            VI::write_display_configuration(
                DisplayConfiguration::zero()
                    .with_enable(true)
                    .with_interlace(Interlace::Interlaced)
                    .with_format(Format::Ntsc),
            );
        }
    }
}
