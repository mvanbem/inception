use crate::codec::Codec;
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

#[derive(Debug)]
pub struct Bgra8;

impl Bgra8 {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        4 * physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        4 * (physical_width * y + x)
    }
}

impl Codec for Bgra8 {
    const FORMAT: TextureFormat = TextureFormat::Bgra8;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 1,
        block_height: 1,
        encoded_block_size: 4,
    };
    type EncodedBlock = [u8; 4];

    fn encode_block(texels: &[u8]) -> [u8; 4] {
        assert_eq!(texels.len(), 4);
        [texels[2], texels[1], texels[0], texels[3]]
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
        let [b, g, r, a]: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
        [r, g, b, a]
    }
}

#[cfg(test)]
mod tests {
    use super::Bgra8;
    use crate::codec::Codec;

    #[test]
    fn encode_block() {
        assert_eq!(
            Bgra8::encode_block(&[0x12, 0x34, 0x56, 0x78]),
            [0x56, 0x34, 0x12, 0x78],
        );
    }

    #[test]
    fn get_texel() {
        const DATA: &[u8] = &[
            0x56, 0x34, 0x12, 0x78, //
            0x78, 0x56, 0x34, 0x9a, //
            0x9a, 0x78, 0x56, 0xbc, //
            0xbc, 0x9a, 0x78, 0xde,
        ];
        assert_eq!(Bgra8::get_texel(2, 2, DATA, 0, 0), [0x12, 0x34, 0x56, 0x78]);
        assert_eq!(Bgra8::get_texel(2, 2, DATA, 1, 0), [0x34, 0x56, 0x78, 0x9a]);
        assert_eq!(Bgra8::get_texel(2, 2, DATA, 0, 1), [0x56, 0x78, 0x9a, 0xbc]);
        assert_eq!(Bgra8::get_texel(2, 2, DATA, 1, 1), [0x78, 0x9a, 0xbc, 0xde]);
    }
}
