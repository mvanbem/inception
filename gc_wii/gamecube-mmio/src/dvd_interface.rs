use core::marker::PhantomData;
use core::mem::transmute;
use core::ptr;

use mvbitfield::prelude::*;

#[repr(C)]
struct RegisterBlock {
    status: Status,
    cover: Cover,
    command_buffer_a: CommandA,
    command_buffer_b: u32,
    command_buffer_c: u32,
    dma_address: u32,
    dma_length: u32,
    control: Control,
    immediate_buffer: ImmediateBuffer,
    config: u32,
}

/// Represents ownership of the DI registers.
pub struct DvdInterface<'reg> {
    _phantom_register_block: PhantomData<&'reg mut RegisterBlock>,
}

impl<'reg> DvdInterface<'reg> {
    const PTR: *mut RegisterBlock = 0xcc006000usize as _;

    /// # Safety
    ///
    /// All calls must have disjoint lifetimes.
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn reborrow(&mut self) -> DvdInterface {
        DvdInterface {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn write_status(&mut self, value: Status) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).status, value) };
    }

    pub fn read_status(&self) -> Status {
        unsafe { ptr::read_volatile(&(*Self::PTR).status) }
    }

    pub fn write_cover(&mut self, value: Cover) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).cover, value) };
    }

    pub fn read_cover(&self) -> Cover {
        unsafe { ptr::read_volatile(&(*Self::PTR).cover) }
    }

    pub fn write_command_buffer_a(&mut self, value: CommandA) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).command_buffer_a, value) };
    }

    pub fn write_command_buffer_b(&mut self, value: u32) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).command_buffer_b, value) };
    }

    pub fn write_command_buffer_c(&mut self, value: u32) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).command_buffer_c, value) };
    }

    pub fn write_dma_address(&mut self, value: u32) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).dma_address, value) };
    }

    pub fn write_dma_length(&mut self, value: u32) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).dma_length, value) };
    }

    pub fn write_control(&mut self, value: Control) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).control, value) };
    }
}

mvbitfield! {
    pub struct Status: u32 {
        pub request_break: 1 as bool,
        pub device_error_mask: 1 as bool,
        pub device_error_interrupt: 1 as bool,
        pub transfer_complete_mask: 1 as bool,
        pub transfer_complete_interrupt: 1 as bool,
        pub break_complete_mask: 1 as bool,
        pub break_complete_interrupt: 1 as bool,
    }
}

mvbitfield! {
    pub struct Cover: u32 {
        pub state: 1 as bool,
        pub mask: 1 as bool,
        pub interrupt: 1 as bool,
    }
}

mvbitfield! {
    pub struct CommandA: u32 {
        pub subcommand2: 16,
        pub subcommand1: 8,
        pub command: 8,
    }
}

mvbitfield! {
    pub struct Control: u32 {
        pub transfer: 1 as bool,
        pub dma: 1 as bool,
        pub access: 1 as Access,
    }
}

#[repr(u8)]
pub enum Access {
    Read = 0,
    Write = 1,
}

impl Access {
    pub const fn from_u1(value: U1) -> Self {
        // SAFETY: Access and U1 have the same layout and valid bit patterns.
        unsafe { transmute(value) }
    }

    pub const fn as_u1(self) -> U1 {
        // SAFETY: Access and U1 have the same layout and valid bit patterns.
        unsafe { transmute(self) }
    }
}

mvbitfield! {
    pub struct ImmediateBuffer: u32 {
        pub reg_val3: 8,
        pub reg_val2: 8,
        pub reg_val1: 8,
        pub reg_val0: 8,
    }
}
