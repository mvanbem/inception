use alloc::vec::Vec;

use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

pub mod bgr8;
pub mod bgra8;
pub mod bgrx8;
pub mod dxt1;
pub mod dxt5;
pub mod dxt_common;
pub mod gx_tf_cmpr;
pub mod gx_tf_i8;
pub mod gx_tf_ia8;
pub mod gx_tf_rgba8;
pub mod rgb8;
pub mod rgba16f;
pub mod rgba8;

pub(crate) trait Codec {
    const FORMAT: TextureFormat;
    const METRICS: BlockMetrics;

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
}

pub(crate) trait DynCodec {
    fn format(&self) -> TextureFormat;

    fn metrics(&self) -> BlockMetrics;

    /// texels: RGBA bytes, row major order
    fn encode_block(&self, texels: &[u8], dst: &mut Vec<u8>);

    fn get_texel(
        &self,
        physical_width: usize,
        physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4];
}

impl<C: Codec> DynCodec for C {
    fn format(&self) -> TextureFormat {
        C::FORMAT
    }

    fn metrics(&self) -> BlockMetrics {
        C::METRICS
    }

    fn encode_block(&self, texels: &[u8], dst: &mut Vec<u8>) {
        dst.extend_from_slice(C::encode_block(texels).as_ref());
    }

    fn get_texel(
        &self,
        physical_width: usize,
        physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4] {
        C::get_texel(physical_width, physical_height, data, x, y)
    }
}
