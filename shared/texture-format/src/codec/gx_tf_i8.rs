use alloc::vec::Vec;

use crate::codec::Codec;
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct GxTfI8;

impl GxTfI8 {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        assert_eq!(physical_width % 8, 0);
        assert_eq!(physical_height % 4, 0);
        physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        assert_eq!(physical_width % 8, 0);
        let blocks_wide = physical_width / 8;
        let coarse_x = x / 8;
        let coarse_y = y / 4;
        let fine_x = x % 8;
        let fine_y = y % 4;
        32 * (blocks_wide * coarse_y + coarse_x) + (8 * fine_y + fine_x)
    }
}

impl Codec for GxTfI8 {
    const FORMAT: TextureFormat = TextureFormat::GxTfI8;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 8,
        block_height: 4,
        encoded_block_size: 32,
    };
    type EncodedBlock = [u8; 32];

    fn encode_block(texels: &[u8]) -> [u8; 32] {
        assert_eq!(texels.len(), 128);

        let mut encoded = Vec::with_capacity(32);
        for y in 0..4 {
            for x in 0..8 {
                encoded.push(
                    ((texels[4 * (8 * y + x)] as u16
                        + texels[4 * (8 * y + x) + 1] as u16
                        + texels[4 * (8 * y + x) + 2] as u16)
                        / 3) as u8,
                );
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
        let i = data[offset];
        [i, i, i, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::GxTfI8;
    use crate::codec::Codec;

    #[test]
    fn encode_block() {
        assert_eq!(
            GxTfI8::encode_block(&[
                0, 0, 0, 0, 1, 1, 1, 0, 2, 2, 2, 0, 3, 3, 3, 0, //
                4, 4, 4, 0, 5, 5, 5, 0, 6, 6, 6, 0, 7, 7, 7, 0, //
                8, 8, 8, 0, 9, 9, 9, 0, 10, 10, 10, 0, 11, 11, 11, 0, //
                12, 12, 12, 0, 13, 13, 13, 0, 14, 14, 14, 0, 15, 15, 15, 0, //
                16, 16, 16, 0, 17, 17, 17, 0, 18, 18, 18, 0, 19, 19, 19, 0, //
                20, 20, 20, 0, 21, 21, 21, 0, 22, 22, 22, 0, 23, 23, 23, 0, //
                24, 24, 24, 0, 25, 25, 25, 0, 26, 26, 26, 0, 27, 27, 27, 0, //
                28, 28, 28, 0, 29, 29, 29, 0, 30, 30, 30, 0, 31, 31, 31, 0, //
            ]),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, //
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, //
            ],
        );
    }

    #[test]
    fn get_texel() {
        const DATA: &[u8] = &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, //
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, //
        ];
        assert_eq!(GxTfI8::get_texel(8, 4, DATA, 0, 0), [0, 0, 0, 255]);
        assert_eq!(GxTfI8::get_texel(8, 4, DATA, 1, 3), [25, 25, 25, 255]);
    }
}
