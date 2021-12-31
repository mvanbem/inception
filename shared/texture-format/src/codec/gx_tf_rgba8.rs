use alloc::vec::Vec;

use crate::codec::Codec;
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct GxTfRgba8;

impl GxTfRgba8 {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        assert_eq!(physical_width % 4, 0);
        assert_eq!(physical_height % 4, 0);
        4 * physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        assert_eq!(physical_width % 4, 0);
        let blocks_wide = physical_width / 4;
        let coarse_x = x / 4;
        let coarse_y = y / 4;
        let fine_x = x % 4;
        let fine_y = y % 4;
        64 * (blocks_wide * coarse_y + coarse_x) + 2 * (4 * fine_y + fine_x)
    }
}

impl Codec for GxTfRgba8 {
    const FORMAT: TextureFormat = TextureFormat::GxTfRgba8;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 4,
        block_height: 4,
        encoded_block_size: 64,
    };
    type EncodedBlock = [u8; 64];

    fn encode_block(texels: &[u8]) -> [u8; 64] {
        assert_eq!(texels.len(), 64);

        let mut encoded = Vec::with_capacity(64);
        for y in 0..4 {
            for x in 0..4 {
                encoded.push(texels[4 * (4 * y + x) + 3]);
                encoded.push(texels[4 * (4 * y + x)]);
            }
        }
        for y in 0..4 {
            for x in 0..4 {
                encoded.push(texels[4 * (4 * y + x) + 1]);
                encoded.push(texels[4 * (4 * y + x) + 2]);
            }
        }
        encoded.try_into().unwrap()
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
        [
            data[offset + 1],
            data[offset + 32],
            data[offset + 33],
            data[offset + 0],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::GxTfRgba8;
    use crate::codec::Codec;

    #[test]
    fn encode_block() {
        assert_eq!(
            GxTfRgba8::encode_block(&[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, //
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, //
                32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, //
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            ]),
            [
                3, 0, 7, 4, 11, 8, 15, 12, 19, 16, 23, 20, 27, 24, 31, 28, //
                35, 32, 39, 36, 43, 40, 47, 44, 51, 48, 55, 52, 59, 56, 63, 60, //
                1, 2, 5, 6, 9, 10, 13, 14, 17, 18, 21, 22, 25, 26, 29, 30, //
                33, 34, 37, 38, 41, 42, 45, 46, 49, 50, 53, 54, 57, 58, 61, 62,
            ],
        );
    }

    #[test]
    fn get_texel() {
        const DATA: &[u8] = &[
            3, 0, 7, 4, 11, 8, 15, 12, 19, 16, 23, 20, 27, 24, 31, 28, //
            35, 32, 39, 36, 43, 40, 47, 44, 51, 48, 55, 52, 59, 56, 63, 60, //
            1, 2, 5, 6, 9, 10, 13, 14, 17, 18, 21, 22, 25, 26, 29, 30, //
            33, 34, 37, 38, 41, 42, 45, 46, 49, 50, 53, 54, 57, 58, 61, 62,
        ];
        assert_eq!(GxTfRgba8::get_texel(4, 4, DATA, 0, 0), [0, 1, 2, 3]);
        assert_eq!(GxTfRgba8::get_texel(4, 4, DATA, 1, 3), [52, 53, 54, 55]);
    }
}
