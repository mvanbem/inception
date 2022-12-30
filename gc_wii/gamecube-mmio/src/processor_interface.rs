use mvbitfield::prelude::*;

mmio_device! {
    doc_name: "PI",
    struct_name: ProcessorInterface,
    base: 0xcc003000,
    size: 0x30,
    regs: {
        interrupt_cause: InterruptCause = ro,
        interrupt_mask: InterruptMask = rw,
        unknown1: u32,
        fifo_base_start: u32,
        fifo_base_end: u32,
        fifo_write_ptr: u32,
        unknown2: u32,
        unknown3: u32,
        unknown4: u32,
        reset: u32,
        unknown5: u32,
        di_control: u32 = rw,
    },
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
