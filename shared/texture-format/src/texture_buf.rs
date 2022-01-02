use alloc::vec::Vec;

use crate::codec::bgr8::Bgr8;
use crate::codec::bgra8::Bgra8;
use crate::codec::bgrx8::Bgrx8;
use crate::codec::dxt1::Dxt1;
use crate::codec::dxt5::Dxt5;
use crate::codec::gx_tf_cmpr::{permute_dxt1_for_gamecube, GxTfCmpr};
use crate::codec::gx_tf_i8::GxTfI8;
use crate::codec::gx_tf_ia8::GxTfIa8;
use crate::codec::gx_tf_rgba8::GxTfRgba8;
use crate::codec::rgb8::Rgb8;
use crate::codec::rgba16f::Rgba16f;
use crate::codec::rgba8::Rgba8;
use crate::codec::Codec;
use crate::{TextureFormat, TextureSlice};

#[derive(Clone)]
pub struct TextureBuf {
    pub(crate) format: TextureFormat,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) physical_width: usize,
    pub(crate) physical_height: usize,
    pub(crate) data: Vec<u8>,
}

impl TextureBuf {
    pub fn new(format: TextureFormat, width: usize, height: usize, data: Vec<u8>) -> Self {
        let metrics = format.metrics();
        let blocks_wide = metrics.blocks_wide(width);
        let blocks_high = metrics.blocks_high(height);
        let physical_width = blocks_wide * metrics.block_width;
        let physical_height = blocks_high * metrics.block_height;

        let expected_size = metrics.encoded_block_size * blocks_wide * blocks_high;
        if data.len() != expected_size {
            panic!(
                "Data size mismatch: format={:?} logical={}x{} physical={}x{} expected={} actual={}",
                format,
                width,
                height,
                physical_width,
                physical_height,
                expected_size,
                data.len(),
            );
        }

        Self {
            format,
            width,
            height,
            physical_width,
            physical_height,
            data,
        }
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn physical_width(&self) -> usize {
        self.physical_width
    }

    pub fn physical_height(&self) -> usize {
        self.physical_height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Fetch and decode the texel at the given location.
    ///
    /// # Performance
    ///
    /// This function performs a trait object dispatch.
    pub fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        self.format.dyn_codec().get_texel(
            self.physical_width,
            self.physical_height,
            &self.data,
            x,
            y,
        )
    }

    pub fn as_slice(&self) -> TextureSlice {
        TextureSlice {
            format: self.format,
            x0: 0,
            y0: 0,
            x1: self.width,
            y1: self.height,
            physical_width: self.physical_width,
            physical_height: self.physical_height,
            data: &self.data,
        }
    }

    pub fn transcode(src: TextureSlice, format: TextureFormat) -> Self {
        match (src.format, format, src.data()) {
            // Same format. No transcoding needed.
            (x, y, Some(src_data)) if x == y => Self {
                format,
                width: src.x1,
                height: src.y1,
                physical_width: src.physical_width,
                physical_height: src.physical_height,
                data: src_data.to_vec(),
            },

            // Special cases to preserve DXT color block encoding.
            (TextureFormat::Dxt1, TextureFormat::GxTfCmpr, Some(src_data)) => {
                encode_dxt1_to_gx_tf_cmpr(src.width(), src.height(), src_data)
            }
            (TextureFormat::Dxt5, TextureFormat::GxTfCmpr, Some(src_data)) => {
                encode_dxt5_to_gx_tf_cmpr(src.width(), src.height(), src_data)
            }

            // General case.
            (_, _, _) => Self::transcode_dispatch_src(src, format),
        }
    }

    fn transcode_dispatch_src(src: TextureSlice, format: TextureFormat) -> Self {
        match src.format {
            TextureFormat::Bgr8 => Self::transcode_dispatch_dst::<Bgr8>(src, format),
            TextureFormat::Bgra8 => Self::transcode_dispatch_dst::<Bgra8>(src, format),
            TextureFormat::Bgrx8 => Self::transcode_dispatch_dst::<Bgrx8>(src, format),
            TextureFormat::Dxt1 => Self::transcode_dispatch_dst::<Dxt1>(src, format),
            TextureFormat::Dxt5 => Self::transcode_dispatch_dst::<Dxt5>(src, format),
            TextureFormat::GxTfCmpr => Self::transcode_dispatch_dst::<GxTfCmpr>(src, format),
            TextureFormat::GxTfI8 => Self::transcode_dispatch_dst::<GxTfI8>(src, format),
            TextureFormat::GxTfIa8 => Self::transcode_dispatch_dst::<GxTfIa8>(src, format),
            TextureFormat::GxTfRgba8 => Self::transcode_dispatch_dst::<GxTfRgba8>(src, format),
            TextureFormat::Rgb8 => Self::transcode_dispatch_dst::<Rgb8>(src, format),
            TextureFormat::Rgba16f => Self::transcode_dispatch_dst::<Rgba16f>(src, format),
            TextureFormat::Rgba8 => Self::transcode_dispatch_dst::<Rgba8>(src, format),
        }
    }

    fn transcode_dispatch_dst<C: Codec>(src: TextureSlice, format: TextureFormat) -> Self {
        assert_eq!(src.format, C::FORMAT);
        match format {
            TextureFormat::Bgr8 => Self::transcode_static::<C, Bgr8>(src),
            TextureFormat::Bgra8 => Self::transcode_static::<C, Bgra8>(src),
            TextureFormat::Bgrx8 => Self::transcode_static::<C, Bgrx8>(src),
            TextureFormat::Dxt1 => Self::transcode_static::<C, Dxt1>(src),
            TextureFormat::Dxt5 => Self::transcode_static::<C, Dxt5>(src),
            TextureFormat::GxTfCmpr => Self::transcode_static::<C, GxTfCmpr>(src),
            TextureFormat::GxTfI8 => Self::transcode_static::<C, GxTfI8>(src),
            TextureFormat::GxTfIa8 => Self::transcode_static::<C, GxTfIa8>(src),
            TextureFormat::GxTfRgba8 => Self::transcode_static::<C, GxTfRgba8>(src),
            TextureFormat::Rgb8 => Self::transcode_static::<C, Rgb8>(src),
            TextureFormat::Rgba16f => Self::transcode_static::<C, Rgba16f>(src),
            TextureFormat::Rgba8 => Self::transcode_static::<C, Rgba8>(src),
        }
    }

    fn transcode_static<C: Codec, D: Codec>(src: TextureSlice) -> Self {
        assert_eq!(src.format, C::FORMAT);
        let width = src.width();
        let height = src.height();
        let dst_blocks_wide = D::METRICS.blocks_wide(width);
        let dst_blocks_high = D::METRICS.blocks_high(height);

        // Visit blocks in order.
        let mut data = Vec::new();
        let mut src_texels =
            Vec::with_capacity(D::METRICS.encoded_block_size * dst_blocks_wide * dst_blocks_high);
        for coarse_y in 0..dst_blocks_high {
            for coarse_x in 0..dst_blocks_wide {
                // Gather RGBA texels to be encoded.
                src_texels.clear();
                for fine_y in 0..D::METRICS.block_height {
                    for fine_x in 0..D::METRICS.block_width {
                        let x = D::METRICS.block_width * coarse_x + fine_x;
                        let y = D::METRICS.block_height * coarse_y + fine_y;
                        let rgba = if x < width && y < height {
                            C::get_texel(
                                src.physical_width,
                                src.physical_height,
                                src.data,
                                x + src.x0,
                                y + src.y0,
                            )
                        } else {
                            [0; 4]
                        };
                        src_texels.extend_from_slice(&rgba);
                    }
                }

                // Encode and store the block.
                data.extend_from_slice(D::encode_block(&src_texels).as_ref());
            }
        }

        Self::new(D::FORMAT, width, height, data)
    }
}

fn encode_dxt1_to_gx_tf_cmpr(width: usize, height: usize, src_data: &[u8]) -> TextureBuf {
    let blocks_wide = GxTfCmpr::METRICS.blocks_wide(width);
    let blocks_high = GxTfCmpr::METRICS.blocks_high(height);
    let mut data =
        Vec::with_capacity(GxTfCmpr::METRICS.encoded_block_size * blocks_wide * blocks_high);
    for coarse_y in 0..blocks_high {
        for coarse_x in 0..blocks_wide {
            for fine_y in 0..2 {
                for fine_x in 0..2 {
                    let offset =
                        8 * (2 * blocks_wide * (2 * coarse_y + fine_y) + 2 * coarse_x + fine_x);
                    match src_data.get(offset..offset + 8) {
                        Some(src_block) => data.extend_from_slice(&permute_dxt1_for_gamecube(
                            src_block.try_into().unwrap(),
                        )),
                        None => data.extend_from_slice(&[0; 8]),
                    }
                }
            }
        }
    }
    TextureBuf {
        format: GxTfCmpr::FORMAT,
        width,
        height,
        physical_width: GxTfCmpr::METRICS.block_width * blocks_wide,
        physical_height: GxTfCmpr::METRICS.block_height * blocks_high,
        data,
    }
}

fn encode_dxt5_to_gx_tf_cmpr(width: usize, height: usize, src_data: &[u8]) -> TextureBuf {
    let blocks_wide = GxTfCmpr::METRICS.blocks_wide(width);
    let blocks_high = GxTfCmpr::METRICS.blocks_high(height);
    let mut data =
        Vec::with_capacity(GxTfCmpr::METRICS.encoded_block_size * blocks_wide * blocks_high);
    for coarse_y in 0..blocks_high {
        for coarse_x in 0..blocks_wide {
            for fine_y in 0..2 {
                for fine_x in 0..2 {
                    let offset =
                        16 * (2 * blocks_wide * (2 * coarse_y + fine_y) + 2 * coarse_x + fine_x);
                    match src_data.get(offset + 8..offset + 16) {
                        Some(src_block) => data.extend_from_slice(&permute_dxt1_for_gamecube(
                            src_block.try_into().unwrap(),
                        )),
                        None => data.extend_from_slice(&[0; 8]),
                    }
                }
            }
        }
    }
    TextureBuf {
        format: GxTfCmpr::FORMAT,
        width,
        height,
        physical_width: GxTfCmpr::METRICS.block_width * blocks_wide,
        physical_height: GxTfCmpr::METRICS.block_height * blocks_high,
        data,
    }
}
