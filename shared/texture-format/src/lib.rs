#![no_std]

extern crate alloc;

mod codec;
mod texture_buf;
mod texture_format;
mod texture_slice;

pub use crate::texture_buf::TextureBuf;
pub use crate::texture_format::{BlockMetrics, TextureFormat};
pub use crate::texture_slice::TextureSlice;
