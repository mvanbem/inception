#[doc = r"Register block"]
#[repr(C)]
pub struct RegisterBlock {
    _reserved0: [u8; 0x24],
    #[doc = "0x24 - "]
    pub di_control: crate::Reg<di_control::DI_CONTROL_SPEC>,
}
#[doc = "di_control register accessor: an alias for `Reg<DI_CONTROL_SPEC>`"]
pub type DI_CONTROL = crate::Reg<di_control::DI_CONTROL_SPEC>;
#[doc = ""]
pub mod di_control;
