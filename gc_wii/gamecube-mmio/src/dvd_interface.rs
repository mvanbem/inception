use core::mem::transmute;

use mvbitfield::prelude::*;

mmio_device! {
    doc_name: "DI",
    struct_name: DvdInterface,
    base: 0xcc006000usize,
    size: 0x28,
    regs: {
        status: Status = rw,
        cover: Cover = rw,
        command_buffer_a: CommandA = wo,
        command_buffer_b: u32 = wo,
        command_buffer_c: u32 = wo,
        dma_address: u32 = wo,
        dma_length: u32 = wo,
        control: Control = wo,
        immediate_buffer: ImmediateBuffer = wo,
        config: u32,
    },
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
