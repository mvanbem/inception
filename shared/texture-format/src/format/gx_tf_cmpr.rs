use alloc::vec::Vec;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{Dxt1, DynTextureFormat, TextureFormat};

fn permute_dxt1_for_gamecube(block: [u8; 8]) -> [u8; 8] {
    // NOTE: This function is written as a little endian to big endian conversion, but the
    // resulting transform is its own inverse.

    let mut result = [0; 8];
    let mut result_writer = &mut result[..];

    // The two colors just need to be byte swapped.
    result_writer
        .write_u16::<BigEndian>((&block[..]).read_u16::<LittleEndian>().unwrap())
        .unwrap();
    result_writer
        .write_u16::<BigEndian>((&block[2..]).read_u16::<LittleEndian>().unwrap())
        .unwrap();

    // The lookup table needs its byte order adapted, but also has a mostly-reversed bit
    // order. Mostly because while the two-bit groups are reversed within the word, the
    // groups themselves stay in order.
    let reverse_two_bit_groups = |x: u32| {
        let x = ((x & 0x0000ffff) << 16) | ((x & 0xffff0000) >> 16);
        let x = ((x & 0x00ff00ff) << 8) | ((x & 0xff00ff00) >> 8);
        let x = ((x & 0x0f0f0f0f) << 4) | ((x & 0xf0f0f0f0) >> 4);
        let x = ((x & 0x33333333) << 2) | ((x & 0xcccccccc) >> 2);
        x
    };
    result_writer
        .write_u32::<BigEndian>(reverse_two_bit_groups(
            (&block[4..]).read_u32::<LittleEndian>().unwrap(),
        ))
        .unwrap();

    result
}

#[derive(Debug)]
pub struct GxTfCmpr;

impl GxTfCmpr {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        assert_eq!(physical_width % Self::BLOCK_WIDTH, 0);
        assert_eq!(physical_height % Self::BLOCK_WIDTH, 0);
        physical_width * physical_height / 2
    }
}

impl TextureFormat for GxTfCmpr {
    const BLOCK_WIDTH: usize = 8;
    const BLOCK_HEIGHT: usize = 8;
    const ENCODED_BLOCK_SIZE: usize = 32;
    type EncodedBlock = [u8; 32];

    fn encode_block(texels: &[u8]) -> [u8; 32] {
        assert_eq!(texels.len(), 256);

        let mut encoded = [0; 32];
        let mut encoded_writer = &mut encoded[..];

        // Compress four sub-blocks, each of which is a permuted DXT1 block.
        for coarse_y in 0..2 {
            for coarse_x in 0..2 {
                // Gather source texels. Each horizontal row of four is already contiguous.
                let mut uncompressed = Vec::with_capacity(4 * 4 * 4);
                for fine_y in 0..4 {
                    let offset = 32 * (4 * coarse_y + fine_y) + 16 * coarse_x;
                    uncompressed.extend_from_slice(&texels[offset..offset + 16]);
                }

                // Encode and permute the sub-block.
                encoded_writer[..8].copy_from_slice(&permute_dxt1_for_gamecube(
                    Dxt1::encode_block(&uncompressed),
                ));
                encoded_writer = &mut encoded_writer[8..];
            }
        }

        assert_eq!(encoded_writer.len(), 0);
        encoded
    }

    fn get_texel(
        physical_width: usize,
        physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4] {
        assert_eq!(data.len(), Self::size(physical_width, physical_height));
        let blocks_wide = physical_width / Self::BLOCK_WIDTH;

        let coarse_x = x / Self::BLOCK_WIDTH;
        let coarse_y = y / Self::BLOCK_WIDTH;
        let medium_x = (x / Dxt1::BLOCK_WIDTH) % 2;
        let medium_y = (y / Dxt1::BLOCK_WIDTH) % 2;
        let fine_x = x % Dxt1::BLOCK_WIDTH;
        let fine_y = y % Dxt1::BLOCK_HEIGHT;

        let offset = Self::ENCODED_BLOCK_SIZE * (blocks_wide * coarse_y + coarse_x)
            + Dxt1::ENCODED_BLOCK_SIZE * (2 * medium_y + medium_x);
        Dxt1::get_texel(
            Dxt1::BLOCK_WIDTH,
            Dxt1::BLOCK_HEIGHT,
            &permute_dxt1_for_gamecube(
                data[offset..offset + Dxt1::ENCODED_BLOCK_SIZE]
                    .try_into()
                    .unwrap(),
            ),
            fine_x,
            fine_y,
        )
    }

    fn as_dyn() -> &'static dyn DynTextureFormat {
        &Self
    }
}

#[cfg(test)]
mod tests {
    use super::GxTfCmpr;
    use crate::TextureFormat;

    #[test]
    fn encode_block() {
        // Just check that it doesn't panic.
        let _ = GxTfCmpr::encode_block(&[0; 256]);
    }

    #[test]
    fn get_texel_transparent() {
        // A <= B, so color 3 is transparent black.
        let data = &[
            0b00000000, 0b00000000, // Color A
            0b00000000, 0b00000000, // Color B
            0b11111111, // Index table MSB
            0b11111111, //
            0b11111111, //
            0b11111111, // Index table LSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        // Fetch from opposite corners just to make sure the whole range works.
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [0, 0, 0, 0]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 3, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn get_texel_half() {
        // A <= B, so color 2 is (A + B) / 2
        let data = &[
            0b00000000, 0b00000000, // Color A
            0b11111111, 0b11111111, // Color B
            0b10101010, // Index table MSB
            0b10101010, //
            0b10101010, //
            0b10101010, // Index table LSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [127, 127, 127, 255]);
    }

    #[test]
    fn get_texel_one_third() {
        // A > B, so color 2 is (2A + B) / 3
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b10101010, // Index table MSB
            0b10101010, //
            0b10101010, //
            0b10101010, // Index table LSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [170, 170, 170, 255]);
    }

    #[test]
    fn get_texel_two_thirds() {
        // A > B, so color 3 is (A + 2B) / 3
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b11111111, // Index table MSB
            0b11111111, //
            0b11111111, //
            0b11111111, // Index table LSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [85, 85, 85, 255]);
    }

    #[test]
    fn get_texel_index_table_order() {
        // A > B, so the colors are [255, 0, 170, 85]
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b00011011, // Index table MSB (first row)
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table LSB (last row)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [255, 255, 255, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 1, 0), [0, 0, 0, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 2, 0), [170, 170, 170, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 3, 0), [85, 85, 85, 255]);

        // A > B, so the colors are [255, 0, 170, 85]
        let data = &[
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table MSB (first row)
            0b01000000, //
            0b10000000, //
            0b11000000, // Index table LSB (last row)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [255, 255, 255, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 1), [0, 0, 0, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 2), [170, 170, 170, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 3), [85, 85, 85, 255]);
    }

    #[test]
    fn get_texel_sub_block_order() {
        let data = &[
            // Color 0 is A
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table MSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table LSB
            // A > B, so color 2 is (2A + B) / 3
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b10101010, // Index table MSB
            0b10101010, //
            0b10101010, //
            0b10101010, // Index table LSB
            // A > B, so color 3 is (A + 2B) / 3
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b11111111, // Index table MSB
            0b11111111, //
            0b11111111, //
            0b11111111, // Index table LSB
            // Color 1 is B
            0b11111111, 0b11111111, // Color A
            0b00000000, 0b00000000, // Color B
            0b01010101, // Index table MSB
            0b01010101, //
            0b01010101, //
            0b01010101, // Index table LSB
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [255, 255, 255, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 4, 0), [170, 170, 170, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 4), [85, 85, 85, 255]);
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 4, 4), [0, 0, 0, 255]);
    }

    #[test]
    fn get_texel_color_channels() {
        let data = &[
            0b11111000, 0b00000000, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [255, 0, 0, 255]);

        let data = &[
            0b00000111, 0b11100000, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [0, 255, 0, 255]);

        let data = &[
            0b00000000, 0b00011111, // Color A
            0b00000000, 0b00000000, // Color B
            0b00000000, // Index table LSB
            0b00000000, //
            0b00000000, //
            0b00000000, // Index table MSB
            0, 0, 0, 0, 0, 0, 0, 0, // Block 1 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 2 (ignored)
            0, 0, 0, 0, 0, 0, 0, 0, // Block 3 (ignored)
        ];
        assert_eq!(GxTfCmpr::get_texel(8, 8, data, 0, 0), [0, 0, 255, 255]);
    }
}
