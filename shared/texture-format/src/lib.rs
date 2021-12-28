#![no_std]

extern crate alloc;

mod any_texture_buf;
mod any_texture_slice;
mod format;
mod texture_buf;
mod texture_slice;
mod traits;

pub use crate::any_texture_buf::AnyTextureBuf;
pub use crate::any_texture_slice::AnyTextureSlice;
pub use crate::format::bgr8::Bgr8;
pub use crate::format::dxt1::Dxt1;
pub use crate::format::dxt5::Dxt5;
pub use crate::format::gx_tf_cmpr::GxTfCmpr;
pub use crate::format::gx_tf_rgba8::GxTfRgba8;
pub use crate::format::rgb8::Rgb8;
pub use crate::format::rgba8::Rgba8;
pub use crate::texture_buf::TextureBuf;
pub use crate::texture_slice::TextureSlice;
pub use crate::traits::{AnyTexture, DynTextureFormat, Texture, TextureFormat, TextureFormatExt};
