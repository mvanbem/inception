use core::fmt::Debug;

use crate::{AnyTextureSlice, TextureSlice};

pub trait AnyTexture {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn get_texel(&self, x: usize, y: usize) -> [u8; 4];
    fn as_slice(&self) -> AnyTextureSlice;
    fn dyn_format(&self) -> &'static dyn DynTextureFormat;
}

pub trait Texture<F> {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn get_texel(&self, x: usize, y: usize) -> [u8; 4];
    fn as_slice(&self) -> TextureSlice<F>;
}

pub trait TextureFormat {
    const BLOCK_WIDTH: usize;
    const BLOCK_HEIGHT: usize;
    const ENCODED_BLOCK_SIZE: usize;
    type EncodedBlock: AsRef<[u8]>;

    /// texels: RGBA bytes, row major order
    fn encode_block(texels: &[u8]) -> Self::EncodedBlock;

    fn get_texel(
        physical_width: usize,
        physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4];

    fn as_dyn() -> &'static dyn DynTextureFormat;
}

pub trait DynTextureFormat: Debug {
    fn block_width(&self) -> usize;
    fn block_height(&self) -> usize;
    fn encoded_block_size(&self) -> usize;
}

impl<F: Debug + TextureFormat> DynTextureFormat for F {
    fn block_width(&self) -> usize {
        Self::BLOCK_WIDTH
    }

    fn block_height(&self) -> usize {
        Self::BLOCK_HEIGHT
    }

    fn encoded_block_size(&self) -> usize {
        Self::ENCODED_BLOCK_SIZE
    }
}

pub trait TextureFormatExt: TextureFormat {
    fn blocks_wide(width: usize) -> usize {
        (width + Self::BLOCK_WIDTH - 1) / Self::BLOCK_WIDTH
    }

    fn blocks_high(height: usize) -> usize {
        (height + Self::BLOCK_HEIGHT - 1) / Self::BLOCK_HEIGHT
    }

    /// Rounds the given width up to the next block boundary.
    fn physical_width(width: usize) -> usize {
        Self::blocks_wide(width) * Self::BLOCK_WIDTH
    }

    /// Rounds the given height up to the next block boundary.
    fn physical_height(height: usize) -> usize {
        Self::blocks_high(height) * Self::BLOCK_HEIGHT
    }

    fn encoded_size(width: usize, height: usize) -> usize {
        Self::ENCODED_BLOCK_SIZE * Self::blocks_wide(width) * Self::blocks_high(height)
    }
}

impl<T: TextureFormat> TextureFormatExt for T {}
