use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;

use mvbitfield::prelude::*;

#[repr(C)]
pub struct RegisterBlock {
    status: Status,
    control: Control,
    _padding_todo_there_are_more_registers: [u16; 62],
}

const _: () = assert!(size_of::<RegisterBlock>() == 0x80);

/// Represents ownership of the CP registers.
pub struct CommandProcessor<'reg> {
    _phantom_register_block: PhantomData<&'reg mut RegisterBlock>,
}

impl<'reg> CommandProcessor<'reg> {
    const PTR: *mut RegisterBlock = 0xcc000000usize as _;

    /// # Safety
    ///
    /// All calls must have disjoint lifetimes.
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn reborrow(&mut self) -> CommandProcessor {
        CommandProcessor {
            _phantom_register_block: PhantomData,
        }
    }

    pub fn read_status(&self) -> Status {
        unsafe { ptr::read_volatile(&(*Self::PTR).status) }
    }

    pub fn write_status(&mut self, value: Status) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).status, value) };
    }

    pub fn read_control(&self) -> Control {
        unsafe { ptr::read_volatile(&(*Self::PTR).control) }
    }

    pub fn write_control(&mut self, value: Control) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).control, value) };
    }
}

mvbitfield! {
    pub struct Status: u16 {
        pub fifo_overflow: 1 as bool,
        pub fifo_underflow: 1 as bool,
        pub is_read_idle: 1 as bool,
        pub is_command_idle: 1 as bool,
        pub breakpoint_interrupt: 1 as bool,
    }
}

mvbitfield! {
    pub struct Control: u16 {
        pub fifo_read_enable: 1 as bool,
        pub cp_interrupt_enable: 1 as bool, // TODO: clarify
        pub fifo_overflow_interrupt_enable: 1 as bool,
        pub fifo_underflow_interrupt_enable: 1 as bool,
        pub fifo_link_enable: 1 as bool,
        pub breakpoint_interrupt_enable: 1 as bool,
    }
}
