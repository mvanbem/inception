#![feature(allocator_api)]
#![no_std]

use core::intrinsics::transmute;
use core::mem::MaybeUninit;
use core::sync::atomic::{compiler_fence, Ordering};

use aligned::{Aligned, A32};
use ogc_sys::DCInvalidateRange;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum DvdError {
    #[snafu(display("DVD Error"))]
    Placeholder,
}

pub fn inquiry() -> Result<[u8; 32], DvdError> {
    let mut aligned = Aligned::<A32, _>([0; 32]);
    dma_read_command([0x12000000, 0, 0x20], &mut *aligned)?;
    Ok(*aligned)
}

pub fn read_disc_id() -> Result<[u8; 32], DvdError> {
    let mut aligned = Aligned::<A32, _>([0; 32]);
    dma_read_command([0xa8000040, 0, aligned.len() as u32], &mut *aligned)?;
    Ok(*aligned)
}

pub fn read(offset: usize, buf: &mut [u8]) -> Result<(), DvdError> {
    assert_eq!(offset % 4, 0);
    dma_read_command([0xa8000000, (offset / 4) as u32, buf.len() as u32], buf)
}

pub fn read_maybe_uninit(offset: usize, buf: &mut [MaybeUninit<u8>]) -> Result<(), DvdError> {
    assert_eq!(offset % 4, 0);
    dma_read_command_maybe_uninit([0xa8000000, (offset / 4) as u32, buf.len() as u32], buf)
}

pub fn wait_for_cover(open: bool) {
    unsafe {
        let di: &'static _ = &*gamecube_peripheral_access::DI::PTR;

        // Disable cover interrupts and acknowledge any pending interrupt.
        di.cover.write_with_zero(|w| {
            w.mask().clear_bit();
            w.ack().set_bit()
        });

        // Wait for the cover state to match the requested state.
        while open != di.cover.read().state().bit() {}
    }
}

pub fn reset() {
    unsafe {
        let pi: &'static _ = &*gamecube_peripheral_access::PI::PTR;

        // Perform a hard reset. I'm not sure what the individual bits or writes do.
        pi.di_control.modify(|r, w| w.bits((r.bits() & !4) | 1));
        pi.di_control.modify(|r, w| w.bits(r.bits() | 5));
        libc::usleep(115000);
    }
}

fn dma_read_command(command: [u32; 3], buf: &mut [u8]) -> Result<(), DvdError> {
    dma_read_command_maybe_uninit(command, unsafe { transmute(buf) })
}

fn dma_read_command_maybe_uninit(
    command: [u32; 3],
    buf: &mut [MaybeUninit<u8>],
) -> Result<(), DvdError> {
    unsafe {
        let di: &'static _ = &*gamecube_peripheral_access::DI::PTR;

        // Disable DI interrupts and acknowledge all pending interrupts.
        di.status.write_with_zero(|w| {
            w.break_complete_mask().clear_bit();
            w.transfer_complete_mask().clear_bit();
            w.device_error_mask().clear_bit();
            w.ack_break_complete().set_bit();
            w.ack_transfer_complete().set_bit();
            w.ack_device_error().set_bit()
        });

        // Set the inquiry command.
        di.command_buffer0.write_with_zero(|w| w.bits(command[0]));
        di.command_buffer1.write_with_zero(|w| w.bits(command[1]));
        di.command_buffer2.write_with_zero(|w| w.bits(command[2]));

        // Point to the buffer.
        assert_eq!(buf.as_mut_ptr() as usize % 32, 0);
        assert_eq!(buf.len() % 32, 0);
        DCInvalidateRange(buf.as_mut_ptr() as _, buf.len() as u32);
        di.dma_address
            .write_with_zero(|w| w.bits(buf.as_mut_ptr() as u32));
        di.dma_length.write_with_zero(|w| w.bits(buf.len() as u32));

        // Fence before the transfer starts because the compiler can't see DMA.
        compiler_fence(Ordering::SeqCst);

        // Start the transfer.
        di.control.write_with_zero(|w| {
            w.access().read();
            w.dma().set_bit();
            w.start_transfer().set_bit()
        });

        loop {
            let status = di.status.read();
            if status.device_error_asserted().bit() {
                di.status
                    .write_with_zero(|w| w.ack_device_error().set_bit());
                return Err(DvdError::Placeholder);
            }
            if status.transfer_complete_asserted().bit() {
                di.status
                    .write_with_zero(|w| w.ack_transfer_complete().set_bit());
                break;
            }
        }

        // Fence after the transfer completes because the compiler can't see DMA.
        compiler_fence(Ordering::SeqCst);

        Ok(())
    }
}
