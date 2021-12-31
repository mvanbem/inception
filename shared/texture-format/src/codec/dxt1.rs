use stb_dxt::{stb_compress_dxt_block, STB_DXT_NORMAL};

use crate::codec::{dxt_common, Codec};
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct Dxt1;

impl Codec for Dxt1 {
    const FORMAT: TextureFormat = TextureFormat::Dxt1;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 4,
        block_height: 4,
        encoded_block_size: 8,
    };
    type EncodedBlock = [u8; 8];

    fn encode_block(texels: &[u8]) -> [u8; 8] {
        assert_eq!(texels.len(), 64);

        let mut compressed = [0; 8];
        unsafe {
            stb_compress_dxt_block(compressed.as_mut_ptr(), texels.as_ptr(), 0, STB_DXT_NORMAL);
        }
        compressed
    }

    fn get_texel(
        physical_width: usize,
        _physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4] {
        assert_eq!(physical_width % 4, 0);
        let blocks_wide = physical_width / 4;

        let coarse_x = x / 4;
        let coarse_y = y / 4;
        let fine_x = x % 4;
        let fine_y = y % 4;

        let offset = 8 * (blocks_wide * coarse_y + coarse_x);
        let block: &[u8; 8] = data[offset..offset + 8].try_into().unwrap();

        dxt_common::block_color(block, fine_x, fine_y)
    }
}

#[cfg(test)]
mod tests {
    use super::Dxt1;
    use crate::codec::Codec;

    #[test]
    fn encode_block() {
        // Just check that it doesn't panic.
        let _ = Dxt1::encode_block(&[0; 64]);
    }

    #[test]
    fn get_texel_transparent() {
        // A <= B, so color 3 is transparent black.
        let data = &[
            0b00000000, 0b00000000, // Color A
            0b00000000, 0b00000000, // Color B
            0b11111111, // Index table LSB
            0b11111111, //
            0b11111111, //
            0b11111111, // Index table MSB
        ];
        // Fetch from opposite corners just to make sure the whole range works.
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [0, 0, 0, 0]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 3, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn get_texel_half() {
        // A <= B, so color 2 is (A + B) / 2
        let data = &[
            0b00000000, 0b00000000, // Color A
            0b11111111, 0b11111111, // Color B
            0b10101010, // Index table LSB
            0b10101010, //
            0b10101010, //
            0b10101010, // Index table MSB
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [127, 127, 127, 255]);
    }

    #[test]
    fn get_texel_one_third() {
        // A > B, so color 2 is (2A + B) / 3
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b10101010, // Index table LSB
            0b10101010, //
            0b10101010, //
            0b10101010, // Index table MSB
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [170, 170, 170, 255]);
    }

    #[test]
    fn get_texel_two_thirds() {
        // A > B, so color 3 is (A + 2B) / 3
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b11111111, // Index table LSB
            0b11111111, //
            0b11111111, //
            0b11111111, // Index table MSB
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [85, 85, 85, 255]);
    }

    #[test]
    fn get_texel_index_table_order() {
        // A > B, so the colors are [255, 0, 170, 85]
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b11100100, // Index table LSB (first row)
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB (last row)
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [255, 255, 255, 255]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 1, 0), [0, 0, 0, 255]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 2, 0), [170, 170, 170, 255]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 3, 0), [85, 85, 85, 255]);

        // A > B, so the colors are [255, 0, 170, 85]
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB (first row)
            0b00000001, //
            0b00000010, //
            0b00000011, // Index table MSB (last row)
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [255, 255, 255, 255]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 1), [0, 0, 0, 255]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 2), [170, 170, 170, 255]);
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 3), [85, 85, 85, 255]);
    }

    #[test]
    fn get_texel_color_channels() {
        let data = &[
            0b00000000, 0b11111000, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [255, 0, 0, 255]);

        let data = &[
            0b11100000, 0b00000111, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [0, 255, 0, 255]);

        let data = &[
            0b00011111, 0b00000000, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB
        ];
        assert_eq!(Dxt1::get_texel(4, 4, data, 0, 0), [0, 0, 255, 255]);
    }
}
