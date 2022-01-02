use crate::codec::bgr8::Bgr8;
use crate::codec::bgra8::Bgra8;
use crate::codec::bgrx8::Bgrx8;
use crate::codec::dxt1::Dxt1;
use crate::codec::dxt5::Dxt5;
use crate::codec::gx_tf_cmpr::GxTfCmpr;
use crate::codec::gx_tf_i8::GxTfI8;
use crate::codec::gx_tf_ia8::GxTfIa8;
use crate::codec::gx_tf_rgba8::GxTfRgba8;
use crate::codec::rgb8::Rgb8;
use crate::codec::rgba16f::Rgba16f;
use crate::codec::rgba8::Rgba8;
use crate::codec::DynCodec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Bgr8,
    Bgra8,
    Bgrx8,
    Dxt1,
    Dxt5,
    GxTfCmpr,
    GxTfI8,
    GxTfIa8,
    GxTfRgba8,
    Rgb8,
    Rgba16f,
    Rgba8,
}

#[derive(Clone, Copy, Debug)]
pub struct BlockMetrics {
    pub block_width: usize,
    pub block_height: usize,
    pub encoded_block_size: usize,
}

impl BlockMetrics {
    pub fn blocks_wide(self, width: usize) -> usize {
        (width + self.block_width - 1) / self.block_width
    }

    pub fn blocks_high(self, height: usize) -> usize {
        (height + self.block_height - 1) / self.block_height
    }

    /// Rounds the given width up to the next block boundary.
    pub fn physical_width(self, width: usize) -> usize {
        self.blocks_wide(width) * self.block_width
    }

    /// Rounds the given height up to the next block boundary.
    pub fn physical_height(self, height: usize) -> usize {
        self.blocks_high(height) * self.block_height
    }

    pub fn encoded_size(self, width: usize, height: usize) -> usize {
        self.encoded_block_size * self.blocks_wide(width) * self.blocks_high(height)
    }
}

impl TextureFormat {
    pub(crate) fn dyn_codec(self) -> &'static dyn DynCodec {
        match self {
            Self::Bgr8 => &Bgr8,
            Self::Bgra8 => &Bgra8,
            Self::Bgrx8 => &Bgrx8,
            Self::Dxt1 => &Dxt1,
            Self::Dxt5 => &Dxt5,
            Self::GxTfCmpr => &GxTfCmpr,
            Self::GxTfI8 => &GxTfI8,
            Self::GxTfIa8 => &GxTfIa8,
            Self::GxTfRgba8 => &GxTfRgba8,
            Self::Rgb8 => &Rgb8,
            Self::Rgba16f => &Rgba16f,
            Self::Rgba8 => &Rgba8,
        }
    }

    pub fn metrics(self) -> BlockMetrics {
        self.dyn_codec().metrics()
    }
}
