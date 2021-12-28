use crate::{DynTextureFormat, TextureFormat};

#[derive(Debug)]
pub struct Bgr8;

impl Bgr8 {
    fn size(physical_width: usize, physical_height: usize) -> usize {
        3 * physical_width * physical_height
    }

    fn texel_offset(physical_width: usize, x: usize, y: usize) -> usize {
        3 * (physical_width * y + x)
    }
}

impl TextureFormat for Bgr8 {
    const BLOCK_WIDTH: usize = 1;
    const BLOCK_HEIGHT: usize = 1;
    const ENCODED_BLOCK_SIZE: usize = 3;
    type EncodedBlock = [u8; 3];

    fn encode_block(texels: &[u8]) -> [u8; 3] {
        assert_eq!(texels.len(), 4);
        [texels[2], texels[1], texels[0]]
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
        let [b, g, r]: [u8; 3] = data[offset..offset + 3].try_into().unwrap();
        [r, g, b, 255]
    }

    fn as_dyn() -> &'static dyn DynTextureFormat {
        &Self
    }
}

#[cfg(test)]
mod tests {
    use super::Bgr8;
    use crate::TextureFormat;

    #[test]
    fn encode_block() {
        assert_eq!(
            Bgr8::encode_block(&[0x12, 0x34, 0x56, 0x78]),
            [0x56, 0x34, 0x12],
        );
    }

    #[test]
    fn get_texel() {
        const DATA: &[u8] = &[
            0x56, 0x34, 0x12, //
            0x78, 0x56, 0x34, //
            0x9a, 0x78, 0x56, //
            0xbc, 0x9a, 0x78,
        ];
        assert_eq!(Bgr8::get_texel(2, 2, DATA, 0, 0), [0x12, 0x34, 0x56, 0xff]);
        assert_eq!(Bgr8::get_texel(2, 2, DATA, 1, 0), [0x34, 0x56, 0x78, 0xff]);
        assert_eq!(Bgr8::get_texel(2, 2, DATA, 0, 1), [0x56, 0x78, 0x9a, 0xff]);
        assert_eq!(Bgr8::get_texel(2, 2, DATA, 1, 1), [0x78, 0x9a, 0xbc, 0xff]);
    }
}
