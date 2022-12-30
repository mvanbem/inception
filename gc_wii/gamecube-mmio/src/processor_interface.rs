use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;

use mvbitfield::prelude::*;

#[repr(C)]
pub struct RegisterBlock {
    interrupt_cause: InterruptCause,
    interrupt_mask: InterruptMask,
    _unknown1: u32,
    _fifo_base_start: u32,
    _fifo_base_end: u32,
    _fifo_write_ptr: u32,
    _unknown2: u32,
    _unknown3: u32,
    _unknown4: u32,
    _reset: u32,
    _unknown5: u32,
    di_control: u32,
}

const _: () = assert!(size_of::<RegisterBlock>() == 48);

/// Represents ownership of the PI registers.
pub struct ProcessorInterface<'reg> {
    _phantom_register_block: PhantomData<&'reg mut RegisterBlock>,
}

impl<'reg> ProcessorInterface<'reg> {
    const PTR: *mut RegisterBlock = 0xcc003000usize as _;

    /// # Safety
    ///
    /// All calls must have disjoint lifetimes.
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn reborrow(&mut self) -> ProcessorInterface {
        ProcessorInterface {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn read_interrupt_cause(&self) -> InterruptCause {
        unsafe { ptr::read_volatile(&(*Self::PTR).interrupt_cause) }
    }

    pub fn write_interrupt_cause(&mut self, value: InterruptCause) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).interrupt_cause, value) };
    }

    pub fn read_interrupt_mask(&self) -> InterruptMask {
        unsafe { ptr::read_volatile(&(*Self::PTR).interrupt_mask) }
    }

    pub fn write_interrupt_mask(&mut self, value: InterruptMask) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).interrupt_mask, value) };
    }

    pub fn read_di_control(&self) -> u32 {
        unsafe { ptr::read_volatile(&(*Self::PTR).di_control) }
    }

    pub fn write_di_control(&mut self, value: u32) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).di_control, value) };
    }

    pub fn modify_di_control(&mut self, f: impl FnOnce(u32) -> u32) {
        self.write_di_control(f(self.read_di_control()));
    }
}

mvbitfield! {
    pub struct InterruptCause: u32 {
        pub interrupts: 14 as Interrupts,
        _reserved: 2,
        pub reset_switch_is_pressed: 1 as bool,
    }
}

mvbitfield! {
    pub struct InterruptMask: u32 {
        pub interrupts: 14 as Interrupts,
    }
}

mvbitfield! {
    pub struct Interrupts: U14 {
        pub gp_error: 1 as bool,
        pub reset_switch: 1 as bool,
        pub dvd: 1 as bool,
        pub serial: 1 as bool,
        pub exi: 1 as bool,
        pub streaming: 1 as bool,
        pub dsp: 1 as bool,
        pub memory_interface: 1 as bool,
        pub video_interface: 1 as bool,
        pub gp_token: 1 as bool,
        pub gp_finish: 1 as bool,
        pub command_processor: 1 as bool,
        pub external_debugger: 1 as bool,
        pub high_speed_port: 1 as bool,
    }
}

impl Interrupts {
    pub const fn all() -> Self {
        Self::zero()
            .with_gp_error(true)
            .with_reset_switch(true)
            .with_dvd(true)
            .with_serial(true)
            .with_exi(true)
            .with_streaming(true)
            .with_dsp(true)
            .with_memory_interface(true)
            .with_video_interface(true)
            .with_gp_token(true)
            .with_gp_finish(true)
            .with_command_processor(true)
            .with_external_debugger(true)
            .with_high_speed_port(true)
    }
}
