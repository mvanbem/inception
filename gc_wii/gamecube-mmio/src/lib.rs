#![no_std]

#[macro_use]
mod macros;

pub mod command_processor;
pub mod dvd_interface;
pub mod processor_interface;
pub mod video_interface;

mod permission;
mod uninterruptible;

pub use crate::permission::PermissionRoot;
pub use crate::uninterruptible::{uninterruptible, Uninterruptible};
