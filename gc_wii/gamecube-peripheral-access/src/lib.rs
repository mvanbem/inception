#![doc = "Peripheral access API for GAMECUBE microcontrollers (generated using svd2rust v0.20.0 ( ))\n\nYou can find an overview of the generated API [here].\n\nAPI features to be included in the [next]
svd2rust release can be generated by cloning the svd2rust [repository], checking out the above commit, and running `cargo doc --open`.\n\n[here]: https://docs.rs/svd2rust/0.20.0/svd2rust/#peripheral-api\n[next]: https://github.com/rust-embedded/svd2rust/blob/master/CHANGELOG.md#unreleased\n[repository]: https://github.com/rust-embedded/svd2rust"]
#![deny(const_err)]
#![deny(dead_code)]
#![deny(improper_ctypes)]
#![deny(missing_docs)]
#![deny(no_mangle_generic_items)]
#![deny(non_shorthand_field_patterns)]
#![deny(overflowing_literals)]
#![deny(path_statements)]
#![deny(patterns_in_fns_without_body)]
#![deny(private_in_public)]
#![deny(unconditional_recursion)]
#![deny(unused_allocation)]
#![deny(unused_comparisons)]
#![deny(unused_parens)]
#![deny(while_true)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![no_std]
use core::marker::PhantomData;
use core::ops::Deref;
#[allow(unused_imports)]
use generic::*;
#[doc = r"Common register and bit access and modify traits"]
pub mod generic;
#[doc = "Command Processor"]
pub struct CP {
    _marker: PhantomData<*const ()>,
}
unsafe impl Send for CP {}
impl CP {
    #[doc = r"Pointer to the register block"]
    pub const PTR: *const cp::RegisterBlock = 0xcc00_0000 as *const _;
    #[doc = r"Return the pointer to the register block"]
    #[inline(always)]
    pub const fn ptr() -> *const cp::RegisterBlock {
        Self::PTR
    }
}
impl Deref for CP {
    type Target = cp::RegisterBlock;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*Self::PTR }
    }
}
impl core::fmt::Debug for CP {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("CP").finish()
    }
}
#[doc = "Command Processor"]
pub mod cp;
#[doc = "Processor Interface"]
pub struct PI {
    _marker: PhantomData<*const ()>,
}
unsafe impl Send for PI {}
impl PI {
    #[doc = r"Pointer to the register block"]
    pub const PTR: *const pi::RegisterBlock = 0xcc00_3000 as *const _;
    #[doc = r"Return the pointer to the register block"]
    #[inline(always)]
    pub const fn ptr() -> *const pi::RegisterBlock {
        Self::PTR
    }
}
impl Deref for PI {
    type Target = pi::RegisterBlock;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*Self::PTR }
    }
}
impl core::fmt::Debug for PI {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("PI").finish()
    }
}
#[doc = "Processor Interface"]
pub mod pi;
#[doc = "DVD Interface"]
pub struct DI {
    _marker: PhantomData<*const ()>,
}
unsafe impl Send for DI {}
impl DI {
    #[doc = r"Pointer to the register block"]
    pub const PTR: *const di::RegisterBlock = 0xcc00_6000 as *const _;
    #[doc = r"Return the pointer to the register block"]
    #[inline(always)]
    pub const fn ptr() -> *const di::RegisterBlock {
        Self::PTR
    }
}
impl Deref for DI {
    type Target = di::RegisterBlock;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*Self::PTR }
    }
}
impl core::fmt::Debug for DI {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("DI").finish()
    }
}
#[doc = "DVD Interface"]
pub mod di;
#[no_mangle]
static mut DEVICE_PERIPHERALS: bool = false;
#[doc = r"All the peripherals"]
#[allow(non_snake_case)]
pub struct Peripherals {
    #[doc = "CP"]
    pub CP: CP,
    #[doc = "PI"]
    pub PI: PI,
    #[doc = "DI"]
    pub DI: DI,
}
impl Peripherals {
    #[doc = r"Unchecked version of `Peripherals::take`"]
    #[inline]
    pub unsafe fn steal() -> Self {
        DEVICE_PERIPHERALS = true;
        Peripherals {
            CP: CP {
                _marker: PhantomData,
            },
            PI: PI {
                _marker: PhantomData,
            },
            DI: DI {
                _marker: PhantomData,
            },
        }
    }
}
