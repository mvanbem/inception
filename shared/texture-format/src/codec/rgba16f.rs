use half::f16;
use num_traits::float::FloatCore;
use num_traits::AsPrimitive;

use crate::codec::Codec;
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct Rgba16f;

impl Rgba16f {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        8 * physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        8 * (physical_width * y + x)
    }
}

impl Codec for Rgba16f {
    const FORMAT: TextureFormat = TextureFormat::Rgba16f;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 1,
        block_height: 1,
        encoded_block_size: 8,
    };
    type EncodedBlock = [u8; 8];

    fn encode_block(texels: &[u8]) -> [u8; 8] {
        assert_eq!(texels.len(), 4);
        let [r, g, b, a]: [u8; 4] = texels.try_into().unwrap();
        let map = |x| (f16::from(x) / f16::from(255u8)).to_le_bytes();
        let r = map(r);
        let g = map(g);
        let b = map(b);
        let a = map(a);
        [r[0], r[1], g[0], g[1], b[0], b[1], a[0], a[1]]
    }

    fn get_texel(
        physical_width: usize,
        physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4] {
        assert_eq!(data.len(), Self::size(physical_width, physical_height));
        let offset = Self::texel_offset(physical_width, x, y);
        let [r0, r1, g0, g1, b0, b1, a0, a1]: [u8; 8] =
            data[offset..offset + 8].try_into().unwrap();
        let map = |x0, x1| {
            (f16::from_le_bytes([x0, x1]) * f16::from(255u8))
                .round()
                .as_()
        };
        let r = map(r0, r1);
        let g = map(g0, g1);
        let b = map(b0, b1);
        let a = map(a0, a1);
        [r, g, b, a]
    }
}
