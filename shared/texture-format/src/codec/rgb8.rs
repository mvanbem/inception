use crate::codec::Codec;
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct Rgb8;

impl Rgb8 {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        3 * physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        3 * (physical_width * y + x)
    }
}

impl Codec for Rgb8 {
    const FORMAT: TextureFormat = TextureFormat::Rgb8;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 1,
        block_height: 1,
        encoded_block_size: 3,
    };
    type EncodedBlock = [u8; 3];

    fn encode_block(texels: &[u8]) -> [u8; 3] {
        assert_eq!(texels.len(), 4);
        texels[..3].try_into().unwrap()
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
        let [r, g, b]: [u8; 3] = data[offset..offset + 3].try_into().unwrap();
        [r, g, b, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::Rgb8;
    use crate::codec::Codec;

    #[test]
    fn encode_block() {
        assert_eq!(
            Rgb8::encode_block(&[0x12, 0x34, 0x56, 0x78]),
            [0x12, 0x34, 0x56],
        );
    }

    #[test]
    fn get_texel() {
        const DATA: &[u8] = &[
            0x12, 0x34, 0x56, //
            0x34, 0x56, 0x78, //
            0x56, 0x78, 0x9a, //
            0x78, 0x9a, 0xbc,
        ];
        assert_eq!(Rgb8::get_texel(2, 2, DATA, 0, 0), [0x12, 0x34, 0x56, 0xff]);
        assert_eq!(Rgb8::get_texel(2, 2, DATA, 1, 0), [0x34, 0x56, 0x78, 0xff]);
        assert_eq!(Rgb8::get_texel(2, 2, DATA, 0, 1), [0x56, 0x78, 0x9a, 0xff]);
        assert_eq!(Rgb8::get_texel(2, 2, DATA, 1, 1), [0x78, 0x9a, 0xbc, 0xff]);
    }
}
