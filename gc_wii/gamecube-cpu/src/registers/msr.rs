use core::arch::asm;
use core::mem::transmute;

use mvbitfield::prelude::*;

mvbitfield! {
    pub struct MachineState: u32 {
        pub endian: 1 as Endian,
        pub exception_is_recoverable: 1 as bool,
        pub process_is_performance_monitor_marked: 1 as bool,
        _reserved: 1,
        pub data_address_translation_enabled: 1 as bool,
        pub instruction_address_translation_enabled: 1 as bool,
        pub exception_prefix: 1 as ExceptionPrefix,
        _reserved: 1,
        pub floating_point_exception_mode_1: 1,
        pub branch_trace_enabled: 1 as bool,
        pub single_step_trace_enabled: 1 as bool,
        pub floating_point_exception_mode_0: 1,
        pub machine_check_enabled: 1 as bool,
        pub floating_point_enabled: 1 as bool,
        pub privilege_level: 1 as PrivilegeLevel,
        pub external_interrupts_enabled: 1 as bool,
        pub exception_endian: 1 as Endian,
        _reserved: 1,
        pub power_management_enabled: 1 as bool,
    }
}

#[repr(u8)]
pub enum Endian {
    Big = 0,
    Little = 1,
}

impl Endian {
    pub const fn from_u1(value: U1) -> Self {
        // SAFETY: Endian and U1 have the same layout and valid bit patterns.
        unsafe { transmute(value) }
    }

    pub const fn as_u1(self) -> U1 {
        // SAFETY: Endian and U1 have the same layout and valid bit patterns.
        unsafe { transmute(self) }
    }
}

#[repr(u8)]
pub enum ExceptionPrefix {
    _0x000nnnnn = 0,
    _0xfffnnnnn = 1,
}

impl ExceptionPrefix {
    pub const fn from_u1(value: U1) -> Self {
        // SAFETY: ExceptionPrefix and U1 have the same layout and valid bit patterns.
        unsafe { transmute(value) }
    }

    pub const fn as_u1(self) -> U1 {
        // SAFETY: ExceptionPrefix and U1 have the same layout and valid bit patterns.
        unsafe { transmute(self) }
    }
}

#[repr(u8)]
pub enum PrivilegeLevel {
    Supervisor = 0,
    User = 1,
}

impl PrivilegeLevel {
    pub const fn from_u1(value: U1) -> Self {
        // SAFETY: PrivilegeLevel and U1 have the same layout and valid bit patterns.
        unsafe { transmute(value) }
    }

    pub const fn as_u1(self) -> U1 {
        // SAFETY: PrivilegeLevel and U1 have the same layout and valid bit patterns.
        unsafe { transmute(self) }
    }
}

pub fn mfmsr() -> MachineState {
    let result;
    unsafe {
        asm!(
            "mfmsr {r}",
            r = out(reg) result,
            options(nomem, preserves_flags, nostack),
        );
    }
    MachineState::from_u32(result)
}

/// # Safety
///
/// Executing the `mtmsr` instruction can reconfigure address translation, invalidating references
/// or causing them to alias.
pub unsafe fn mtmsr(value: MachineState) {
    asm!(
        "mtmsr {r}",
        r = in(reg) value.as_u32(),
        options(nomem, preserves_flags, nostack),
    );
}

/// # Safety
///
/// Executing the `mtmsr` instruction can reconfigure address translation, invalidating references
/// or causing them to alias.
pub unsafe fn modify_msr(f: impl FnOnce(MachineState) -> MachineState) {
    mtmsr(f(mfmsr()));
}
