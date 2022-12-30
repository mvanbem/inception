use core::marker::PhantomData;
use core::mem::{size_of, transmute};
use core::ptr::{self};

use mvbitfield::prelude::*;

#[repr(C)]
struct RegisterBlock {
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

const _: () = assert!(size_of::<RegisterBlock>() == 0x70);

/// Represents ownership of the VI registers.
pub struct VideoInterface<'reg> {
    _phantom_register_block: PhantomData<&'reg mut RegisterBlock>,
}

impl<'reg> VideoInterface<'reg> {
    const PTR: *mut RegisterBlock = 0xcc002000usize as _;

    /// # Safety
    ///
    /// All calls must have disjoint lifetimes.
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn reborrow(&mut self) -> VideoInterface {
        VideoInterface {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn write_vertical_timing_a(&mut self, value: VerticalTimingA) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).vertical_timing_a, value) };
    }

    pub fn write_display_configuration(&mut self, value: DisplayConfiguration) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_configuration, value) };
    }

    pub fn write_horizontal_timing_a(&mut self, value: HorizontalTimingA) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).horizontal_timing_a, value) };
    }

    pub fn write_horizontal_timing_b(&mut self, value: HorizontalTimingB) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).horizontal_timing_b, value) };
    }

    pub fn write_vertical_timing_b_odd_field(&mut self, value: VerticalTimingB) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).vertical_timing_b_odd_field, value) };
    }

    pub fn write_vertical_timing_b_even_field(&mut self, value: VerticalTimingB) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).vertical_timing_b_even_field, value) };
    }

    pub fn write_burst_blanking_odd_field(&mut self, value: BurstBlankingOddField) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).burst_blanking_odd_field, value) };
    }

    pub fn write_burst_blanking_even_field(&mut self, value: BurstBlankingEvenField) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).burst_blanking_even_field, value) };
    }

    pub fn write_top_left_field_base(&mut self, value: FieldBase) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).top_left_field_base, value) };
    }

    pub fn write_bottom_left_field_base(&mut self, value: FieldBase) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).bottom_left_field_base, value) };
    }

    pub fn write_display_interrupt_0(&mut self, value: DisplayInterrupt) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_interrupt[0], value) };
    }

    pub fn write_display_interrupt_1(&mut self, value: DisplayInterrupt) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_interrupt[1], value) };
    }

    pub fn write_display_interrupt_2(&mut self, value: DisplayInterrupt) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_interrupt[2], value) };
    }

    pub fn write_display_interrupt_3(&mut self, value: DisplayInterrupt) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_interrupt[3], value) };
    }

    pub fn read_display_latch_0(&self) -> DisplayLatch {
        unsafe { ptr::read_volatile(&(*Self::PTR).display_latch[0]) }
    }

    pub fn write_display_latch_0(&mut self, value: DisplayLatch) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_latch[0], value) };
    }

    pub fn read_display_latch_1(&self) -> DisplayLatch {
        unsafe { ptr::read_volatile(&(*Self::PTR).display_latch[1]) }
    }

    pub fn write_display_latch_1(&mut self, value: DisplayLatch) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).display_latch[1], value) };
    }

    pub fn write_horizontal_scaling(&mut self, value: HorizontalScaling) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).horizontal_scaling, value) };
    }

    pub fn write_clock_select(&mut self, value: ClockSelect) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).clock_select, value) };
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
        pub horizontal_position: 11,
        _reserved: 5,
        pub vertical_position: 11,
        _reserved: 1,
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
    k27MHz = 0,
    k54MHz = 1,
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
