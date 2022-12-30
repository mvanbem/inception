#![feature(allocator_api)]
#![no_std]

use core::intrinsics::transmute;
use core::mem::MaybeUninit;
use core::sync::atomic::{compiler_fence, Ordering};

use aligned::{Aligned, A32};
use gamecube_mmio::dvd_interface::*;
use gamecube_mmio::processor_interface::ProcessorInterface;
use gamecube_mmio::uninterruptible::uninterruptible;
use ogc_sys::DCInvalidateRange;
use snafu::Snafu;

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

pub struct DvdDriver<'reg> {
    di: DvdInterface<'reg>,
}

impl<'reg> DvdDriver<'reg> {
    pub fn new(di: DvdInterface<'reg>) -> Self {
        Self { di }
    }

    pub fn inquiry(&mut self) -> Result<[u8; 32], DvdError> {
        let mut aligned = Aligned::<A32, _>([0; 32]);
        self.dma_read_command(
            Command {
                a: CommandA::zero().with_command(0x12),
                b: 0,
                c: 0x20,
            },
            &mut *aligned,
        )?;
        Ok(*aligned)
    }

    pub fn read_disc_id(&mut self) -> Result<[u8; 32], DvdError> {
        let mut aligned = Aligned::<A32, _>([0; 32]);
        self.dma_read_command(
            Command {
                a: CommandA::zero().with_command(0xa8).with_subcommand2(0x0040),
                b: 0,
                c: aligned.len() as u32,
            },
            &mut *aligned,
        )?;
        Ok(*aligned)
    }

    pub fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), DvdError> {
        assert_eq!(offset % 4, 0);
        self.dma_read_command(
            Command {
                a: CommandA::zero().with_command(0xa8),
                b: (offset / 4) as u32,
                c: buf.len() as u32,
            },
            buf,
        )
    }

    pub fn read_maybe_uninit(
        &mut self,
        offset: usize,
        buf: &mut [MaybeUninit<u8>],
    ) -> Result<(), DvdError> {
        assert_eq!(offset % 4, 0);
        self.dma_read_command_maybe_uninit(
            Command {
                a: CommandA::zero().with_command(0xa8),
                b: (offset / 4) as u32,
                c: buf.len() as u32,
            },
            buf,
        )
    }

    pub fn wait_for_cover(&mut self, open: bool) {
        // Disable cover interrupts and acknowledge any pending interrupt.
        self.di
            .write_cover(Cover::zero().with_mask(true).with_interrupt(true));

        // Wait for the cover state to match the requested state.
        while open != self.di.read_cover().state() {}
    }

    pub fn reset(&mut self, pi: ProcessorInterface) {
        uninterruptible(|u| {
            // Perform a hard reset. I'm not sure what the individual bits or writes do.
            pi.modify_di_control(u, |x| (x & !4) | 1);
            pi.modify_di_control(u, |x| x | 5);
        });
        unsafe { libc::usleep(115000) };
    }

    fn dma_read_command(&mut self, command: Command, buf: &mut [u8]) -> Result<(), DvdError> {
        self.dma_read_command_maybe_uninit(command, unsafe { transmute(buf) })
    }

    fn dma_read_command_maybe_uninit(
        &mut self,
        command: Command,
        buf: &mut [MaybeUninit<u8>],
    ) -> Result<(), DvdError> {
        unsafe {
            // Disable DI interrupts and acknowledge all pending interrupts.
            self.di.write_status(
                Status::zero()
                    .with_break_complete_interrupt(true)
                    .with_transfer_complete_interrupt(true)
                    .with_device_error_interrupt(true),
            );

            // Set the inquiry command.
            self.di.write_command_buffer_a(command.a);
            self.di.write_command_buffer_b(command.b);
            self.di.write_command_buffer_c(command.c);

            // Point to the buffer.
            assert_eq!(buf.as_mut_ptr() as usize % 32, 0);
            assert_eq!(buf.len() % 32, 0);
            DCInvalidateRange(buf.as_mut_ptr() as _, buf.len() as u32);
            self.di.write_dma_address(buf.as_mut_ptr() as u32);
            self.di.write_dma_length(buf.len() as u32);

            // Fence before the transfer starts because the compiler can't see DMA.
            compiler_fence(Ordering::SeqCst);

            // Start the transfer.
            self.di.write_control(
                Control::zero()
                    .with_access(Access::Read)
                    .with_dma(true)
                    .with_transfer(true),
            );

            loop {
                let status = self.di.read_status();
                if status.device_error_interrupt() {
                    self.di
                        .write_status(Status::zero().with_device_error_interrupt(true));
                    return Err(DvdError::Placeholder);
                }
                if status.transfer_complete_interrupt() {
                    self.di
                        .write_status(Status::zero().with_transfer_complete_interrupt(true));
                    break;
                }
            }

            // Fence after the transfer completes because the compiler can't see DMA.
            compiler_fence(Ordering::SeqCst);

            Ok(())
        }
    }
}
