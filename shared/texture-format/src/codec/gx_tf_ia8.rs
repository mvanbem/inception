use alloc::vec::Vec;

use crate::codec::Codec;
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct GxTfIa8;

impl GxTfIa8 {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        assert_eq!(physical_width % 4, 0);
        assert_eq!(physical_height % 4, 0);
        physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        assert_eq!(physical_width % 4, 0);
        let blocks_wide = physical_width / 4;
        let coarse_x = x / 4;
        let coarse_y = y / 4;
        let fine_x = x % 4;
        let fine_y = y % 4;
        32 * (blocks_wide * coarse_y + coarse_x) + 2 * (4 * fine_y + fine_x)
    }
}

impl Codec for GxTfIa8 {
    const FORMAT: TextureFormat = TextureFormat::GxTfIa8;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 4,
        block_height: 4,
        encoded_block_size: 32,
    };
    type EncodedBlock = [u8; 32];

    fn encode_block(texels: &[u8]) -> [u8; 32] {
        assert_eq!(texels.len(), 64);

        let mut encoded = Vec::with_capacity(32);
        for y in 0..4 {
            for x in 0..4 {
                let offset = 4 * (4 * y + x);
                let [r, g, b, a]: [u8; 4] = texels[offset..offset + 4].try_into().unwrap();
                let i = ((r as u16 + g as u16 + b as u16) / 3) as u8;
                encoded.push(a);
                encoded.push(i);
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
        let a = data[offset];
        let i = data[offset + 1];
        [i, i, i, a]
    }
}

#[cfg(test)]
mod tests {
    use super::GxTfIa8;
    use crate::codec::Codec;

    #[test]
    fn encode_block() {
        assert_eq!(
            GxTfIa8::encode_block(&[
                0x00, 0x00, 0x00, 0x01, 0x02, 0x02, 0x02, 0x03, //
                0x04, 0x04, 0x04, 0x05, 0x06, 0x06, 0x06, 0x07, //
                0x08, 0x08, 0x08, 0x09, 0x0a, 0x0a, 0x0a, 0x0b, //
                0x0c, 0x0c, 0x0c, 0x0d, 0x0e, 0x0e, 0x0e, 0x0f, //
                0x10, 0x10, 0x10, 0x11, 0x12, 0x12, 0x12, 0x13, //
                0x14, 0x14, 0x14, 0x15, 0x16, 0x16, 0x16, 0x17, //
                0x18, 0x18, 0x18, 0x19, 0x1a, 0x1a, 0x1a, 0x1b, //
                0x1c, 0x1c, 0x1c, 0x1d, 0x1e, 0x1e, 0x1e, 0x1f, //
            ]),
            [
                0x01, 0x00, 0x03, 0x02, 0x05, 0x04, 0x07, 0x06, //
                0x09, 0x08, 0x0b, 0x0a, 0x0d, 0x0c, 0x0f, 0x0e, //
                0x11, 0x10, 0x13, 0x12, 0x15, 0x14, 0x17, 0x16, //
                0x19, 0x18, 0x1b, 0x1a, 0x1d, 0x1c, 0x1f, 0x1e, //
            ],
        );
    }

    #[test]
    fn get_texel() {
        const DATA: &[u8] = &[
            0x01, 0x00, 0x03, 0x02, 0x05, 0x04, 0x07, 0x06, //
            0x09, 0x08, 0x0b, 0x0a, 0x0d, 0x0c, 0x0f, 0x0e, //
            0x11, 0x10, 0x13, 0x12, 0x15, 0x14, 0x17, 0x16, //
            0x19, 0x18, 0x1b, 0x1a, 0x1d, 0x1c, 0x1f, 0x1e, //
        ];
        assert_eq!(
            GxTfIa8::get_texel(8, 4, DATA, 0, 0),
            [0x00, 0x00, 0x00, 0x01],
        );
        assert_eq!(
            GxTfIa8::get_texel(8, 4, DATA, 1, 3),
            [0x1a, 0x1a, 0x1a, 0x1b],
        );
    }
}
