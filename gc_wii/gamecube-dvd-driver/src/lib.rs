#![feature(allocator_api)]
#![no_std]

use core::intrinsics::transmute;
use core::mem::MaybeUninit;
use core::sync::atomic::{compiler_fence, Ordering};

use aligned::{Aligned, A32};
use ogc_sys::DCInvalidateRange;
use snafu::Snafu;

use crate::registers::*;

mod registers {
    use core::mem::transmute;
    use core::ptr;

    use mvbitfield::prelude::*;

    /// Base address 0xcc003000
    #[repr(C)]
    pub struct PI {
        _padding: [u8; 0x24],
        di_control: u32,
    }

    impl PI {
        pub const PTR: *mut Self = 0xcc003000usize as _;

        pub unsafe fn write_di_control(value: u32) {
            ptr::write_volatile(&mut (*Self::PTR).di_control, value);
        }

        pub unsafe fn read_di_control() -> u32 {
            ptr::read_volatile(&(*Self::PTR).di_control)
        }

        pub unsafe fn modify_di_control(f: impl FnOnce(u32) -> u32) {
            Self::write_di_control(f(Self::read_di_control()));
        }
    }

    /// Base address 0xcc006000
    #[repr(C)]
    pub struct DI {
        pub status: Status,
        pub cover: Cover,
        pub command_buffer_a: CommandA,
        pub command_buffer_b: u32,
        pub command_buffer_c: u32,
        pub dma_address: u32,
        pub dma_length: u32,
        pub control: Control,
        pub immediate_buffer: ImmediateBuffer,
        pub config: u32,
    }

    impl DI {
        pub const PTR: *mut Self = 0xcc006000usize as _;

        pub unsafe fn write_status(value: Status) {
            ptr::write_volatile(&mut (*Self::PTR).status, value);
        }

        pub unsafe fn read_status() -> Status {
            ptr::read_volatile(&(*Self::PTR).status)
        }

        pub unsafe fn write_cover(value: Cover) {
            ptr::write_volatile(&mut (*Self::PTR).cover, value);
        }

        pub unsafe fn read_cover() -> Cover {
            ptr::read_volatile(&(*Self::PTR).cover)
        }

        pub unsafe fn write_command_buffer_a(value: CommandA) {
            ptr::write_volatile(&mut (*Self::PTR).command_buffer_a, value);
        }

        pub unsafe fn write_command_buffer_b(value: u32) {
            ptr::write_volatile(&mut (*Self::PTR).command_buffer_b, value);
        }

        pub unsafe fn write_command_buffer_c(value: u32) {
            ptr::write_volatile(&mut (*Self::PTR).command_buffer_c, value);
        }

        pub unsafe fn write_dma_address(value: u32) {
            ptr::write_volatile(&mut (*Self::PTR).dma_address, value);
        }

        pub unsafe fn write_dma_length(value: u32) {
            ptr::write_volatile(&mut (*Self::PTR).dma_length, value);
        }

        pub unsafe fn write_control(value: Control) {
            ptr::write_volatile(&mut (*Self::PTR).control, value);
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
}

struct Command {
    a: CommandA,
    b: u32,
    c: u32,
}

#[derive(Debug, Snafu)]
pub enum DvdError {
    #[snafu(display("DVD Error"))]
    Placeholder,
}

pub fn inquiry() -> Result<[u8; 32], DvdError> {
    let mut aligned = Aligned::<A32, _>([0; 32]);
    dma_read_command(
        Command {
            a: CommandA::zero().with_command(0x12),
            b: 0,
            c: 0x20,
        },
        &mut *aligned,
    )?;
    Ok(*aligned)
}

pub fn read_disc_id() -> Result<[u8; 32], DvdError> {
    let mut aligned = Aligned::<A32, _>([0; 32]);
    dma_read_command(
        Command {
            a: CommandA::zero().with_command(0xa8).with_subcommand2(0x0040),
            b: 0,
            c: aligned.len() as u32,
        },
        &mut *aligned,
    )?;
    Ok(*aligned)
}

pub fn read(offset: usize, buf: &mut [u8]) -> Result<(), DvdError> {
    assert_eq!(offset % 4, 0);
    dma_read_command(
        Command {
            a: CommandA::zero().with_command(0xa8),
            b: (offset / 4) as u32,
            c: buf.len() as u32,
        },
        buf,
    )
}

pub fn read_maybe_uninit(offset: usize, buf: &mut [MaybeUninit<u8>]) -> Result<(), DvdError> {
    assert_eq!(offset % 4, 0);
    dma_read_command_maybe_uninit(
        Command {
            a: CommandA::zero().with_command(0xa8),
            b: (offset / 4) as u32,
            c: buf.len() as u32,
        },
        buf,
    )
}

pub fn wait_for_cover(open: bool) {
    unsafe {
        // Disable cover interrupts and acknowledge any pending interrupt.
        DI::write_cover(Cover::zero().with_mask(true).with_interrupt(true));

        // Wait for the cover state to match the requested state.
        while open != DI::read_cover().state() {}
    }
}

pub fn reset() {
    unsafe {
        // Perform a hard reset. I'm not sure what the individual bits or writes do.
        PI::modify_di_control(|x| (x & !4) | 1);
        PI::modify_di_control(|x| x | 5);
        libc::usleep(115000);
    }
}

fn dma_read_command(command: Command, buf: &mut [u8]) -> Result<(), DvdError> {
    dma_read_command_maybe_uninit(command, unsafe { transmute(buf) })
}

fn dma_read_command_maybe_uninit(
    command: Command,
    buf: &mut [MaybeUninit<u8>],
) -> Result<(), DvdError> {
    unsafe {
        // Disable DI interrupts and acknowledge all pending interrupts.
        DI::write_status(
            Status::zero()
                .with_break_complete_interrupt(true)
                .with_transfer_complete_interrupt(true)
                .with_device_error_interrupt(true),
        );

        // Set the inquiry command.
        DI::write_command_buffer_a(command.a);
        DI::write_command_buffer_b(command.b);
        DI::write_command_buffer_c(command.c);

        // Point to the buffer.
        assert_eq!(buf.as_mut_ptr() as usize % 32, 0);
        assert_eq!(buf.len() % 32, 0);
        DCInvalidateRange(buf.as_mut_ptr() as _, buf.len() as u32);
        DI::write_dma_address(buf.as_mut_ptr() as u32);
        DI::write_dma_length(buf.len() as u32);

        // Fence before the transfer starts because the compiler can't see DMA.
        compiler_fence(Ordering::SeqCst);

        // Start the transfer.
        DI::write_control(
            Control::zero()
                .with_access(Access::Read)
                .with_dma(true)
                .with_transfer(true),
        );

        loop {
            let status = DI::read_status();
            if status.device_error_interrupt() {
                DI::write_status(Status::zero().with_device_error_interrupt(true));
                return Err(DvdError::Placeholder);
            }
            if status.transfer_complete_interrupt() {
                DI::write_status(Status::zero().with_transfer_complete_interrupt(true));
                break;
            }
        }

        // Fence after the transfer completes because the compiler can't see DMA.
        compiler_fence(Ordering::SeqCst);

        Ok(())
    }
}
