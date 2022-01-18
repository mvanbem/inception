#[doc = r"Register block"]
#[repr(C)]
pub struct RegisterBlock {
    #[doc = "0x00 - "]
    pub status: crate::Reg<status::STATUS_SPEC>,
    #[doc = "0x04 - "]
    pub cover: crate::Reg<cover::COVER_SPEC>,
    #[doc = "0x08 - "]
    pub command_buffer0: crate::Reg<command_buffer0::COMMAND_BUFFER0_SPEC>,
    #[doc = "0x0c - "]
    pub command_buffer1: crate::Reg<command_buffer1::COMMAND_BUFFER1_SPEC>,
    #[doc = "0x10 - "]
    pub command_buffer2: crate::Reg<command_buffer2::COMMAND_BUFFER2_SPEC>,
    #[doc = "0x14 - "]
    pub dma_address: crate::Reg<dma_address::DMA_ADDRESS_SPEC>,
    #[doc = "0x18 - "]
    pub dma_length: crate::Reg<dma_length::DMA_LENGTH_SPEC>,
    #[doc = "0x1c - "]
    pub control: crate::Reg<control::CONTROL_SPEC>,
    #[doc = "0x20 - "]
    pub immediate_buffer: crate::Reg<immediate_buffer::IMMEDIATE_BUFFER_SPEC>,
    #[doc = "0x24 - "]
    pub config: crate::Reg<config::CONFIG_SPEC>,
}
#[doc = "status register accessor: an alias for `Reg<STATUS_SPEC>`"]
pub type STATUS = crate::Reg<status::STATUS_SPEC>;
#[doc = ""]
pub mod status;
#[doc = "cover register accessor: an alias for `Reg<COVER_SPEC>`"]
pub type COVER = crate::Reg<cover::COVER_SPEC>;
#[doc = ""]
pub mod cover;
#[doc = "command_buffer0 register accessor: an alias for `Reg<COMMAND_BUFFER0_SPEC>`"]
pub type COMMAND_BUFFER0 = crate::Reg<command_buffer0::COMMAND_BUFFER0_SPEC>;
#[doc = ""]
pub mod command_buffer0;
#[doc = "command_buffer1 register accessor: an alias for `Reg<COMMAND_BUFFER1_SPEC>`"]
pub type COMMAND_BUFFER1 = crate::Reg<command_buffer1::COMMAND_BUFFER1_SPEC>;
#[doc = ""]
pub mod command_buffer1;
#[doc = "command_buffer2 register accessor: an alias for `Reg<COMMAND_BUFFER2_SPEC>`"]
pub type COMMAND_BUFFER2 = crate::Reg<command_buffer2::COMMAND_BUFFER2_SPEC>;
#[doc = ""]
pub mod command_buffer2;
#[doc = "dma_address register accessor: an alias for `Reg<DMA_ADDRESS_SPEC>`"]
pub type DMA_ADDRESS = crate::Reg<dma_address::DMA_ADDRESS_SPEC>;
#[doc = ""]
pub mod dma_address;
#[doc = "dma_length register accessor: an alias for `Reg<DMA_LENGTH_SPEC>`"]
pub type DMA_LENGTH = crate::Reg<dma_length::DMA_LENGTH_SPEC>;
#[doc = ""]
pub mod dma_length;
#[doc = "control register accessor: an alias for `Reg<CONTROL_SPEC>`"]
pub type CONTROL = crate::Reg<control::CONTROL_SPEC>;
#[doc = ""]
pub mod control;
#[doc = "immediate_buffer register accessor: an alias for `Reg<IMMEDIATE_BUFFER_SPEC>`"]
pub type IMMEDIATE_BUFFER = crate::Reg<immediate_buffer::IMMEDIATE_BUFFER_SPEC>;
#[doc = ""]
pub mod immediate_buffer;
#[doc = "config register accessor: an alias for `Reg<CONFIG_SPEC>`"]
pub type CONFIG = crate::Reg<config::CONFIG_SPEC>;
#[doc = ""]
pub mod config;
