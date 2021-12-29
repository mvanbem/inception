use core::marker::PhantomData;
use core::ptr;

use alloc::vec::Vec;

use crate::format::gx_tf_cmpr::permute_dxt1_for_gamecube;
use crate::{
    AnyTexture, AnyTextureSlice, Dxt1, Dxt5, GxTfCmpr, Texture, TextureFormat, TextureFormatExt,
    TextureSlice,
};

pub struct TextureBuf<F> {
    pub(crate) logical_width: usize,
    pub(crate) logical_height: usize,
    pub(crate) physical_width: usize,
    pub(crate) physical_height: usize,
    pub(crate) data: Vec<u8>,
    pub(crate) _phantom_format: PhantomData<*const F>,
}

impl<F: TextureFormat> TextureBuf<F> {
    pub fn new(logical_width: usize, logical_height: usize, data: Vec<u8>) -> Self {
        let physical_width = F::physical_width(logical_width);
        let physical_height = F::physical_height(logical_height);

        let expected_size = F::ENCODED_BLOCK_SIZE
            * (physical_width / F::BLOCK_WIDTH)
            * (physical_height / F::BLOCK_HEIGHT);
        if data.len() != expected_size {
            panic!(
                "Data size mismatch: format={:?} logical={}x{} physical={}x{} expected={} actual={}",
                F::as_dyn(),
                logical_width,
                logical_height,
                physical_width,
                physical_height,
                expected_size,
                data.len(),
            );
        }

        Self {
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            data,
            _phantom_format: PhantomData,
        }
    }

    pub fn physical_width(&self) -> usize {
        self.physical_width
    }

    pub fn physical_height(&self) -> usize {
        self.physical_height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        F::get_texel(self.physical_width, self.physical_height, &self.data, x, y)
    }

    pub fn encode_any<T: AnyTexture>(src: T) -> Self {
        match src.as_slice() {
            AnyTextureSlice::Rgb8(x) => Self::encode(x),
            AnyTextureSlice::Bgr8(x) => Self::encode(x),
            AnyTextureSlice::Rgba8(x) => Self::encode(x),
            AnyTextureSlice::Dxt1(x) => Self::encode(x),
            AnyTextureSlice::Dxt5(x) => Self::encode(x),
            AnyTextureSlice::GxTfRgba8(x) => Self::encode(x),
            AnyTextureSlice::GxTfCmpr(x) => Self::encode(x),
        }
    }

    pub fn encode<T: Texture<G>, G: TextureFormat>(src: T) -> Self {
        if let Some(src_data) = src.data() {
            if ptr::eq(F::as_dyn(), G::as_dyn()) {
                // Same format. No transcoding needed.
                return Self {
                    logical_width: src.width(),
                    logical_height: src.height(),
                    physical_width: F::physical_width(src.width()),
                    physical_height: F::physical_height(src.height()),
                    data: src_data.to_vec(),
                    _phantom_format: PhantomData,
                };
            } else if ptr::eq(G::as_dyn(), Dxt1::as_dyn())
                && ptr::eq(F::as_dyn(), GxTfCmpr::as_dyn())
            {
                return encode_dxt1_to_gx_tf_cmpr(src.width(), src.height(), src_data);
            } else if ptr::eq(G::as_dyn(), Dxt5::as_dyn())
                && ptr::eq(F::as_dyn(), GxTfCmpr::as_dyn())
            {
                return encode_dxt5_to_gx_tf_cmpr(src.width(), src.height(), src_data);
            }
        }

        let mut data = Vec::new();

        let logical_width = src.width();
        let logical_height = src.height();
        let physical_width = F::physical_width(logical_width);
        let physical_height = F::physical_height(logical_height);

        // Visit blocks in order.
        let blocks_wide = physical_width / F::BLOCK_WIDTH;
        let blocks_high = physical_height / F::BLOCK_HEIGHT;
        let mut texels =
            Vec::with_capacity(F::ENCODED_BLOCK_SIZE * F::BLOCK_WIDTH * F::BLOCK_HEIGHT);
        for coarse_y in 0..blocks_high {
            for coarse_x in 0..blocks_wide {
                // Gather RGBA texels to be encoded.
                texels.clear();
                for fine_y in 0..F::BLOCK_HEIGHT {
                    for fine_x in 0..F::BLOCK_WIDTH {
                        let x = F::BLOCK_WIDTH * coarse_x + fine_x;
                        let y = F::BLOCK_HEIGHT * coarse_y + fine_y;
                        let rgba = if x < logical_width && y < logical_height {
                            src.get_texel(x, y)
                        } else {
                            [0; 4]
                        };
                        texels.extend_from_slice(&rgba);
                    }
                }

                // Encode and store the block.
                data.extend_from_slice(F::encode_block(&texels).as_ref());
            }
        }

        Self {
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            data,
            _phantom_format: PhantomData,
        }
    }
}

fn encode_dxt1_to_gx_tf_cmpr<F: TextureFormat>(
    width: usize,
    height: usize,
    src_data: &[u8],
) -> TextureBuf<F> {
    assert!(ptr::eq(F::as_dyn(), GxTfCmpr::as_dyn()));
    let physical_width = GxTfCmpr::physical_width(width);
    let physical_height = GxTfCmpr::physical_height(height);

    let blocks_wide = width / 8;
    let blocks_high = height / 8;
    let mut data = Vec::with_capacity(GxTfCmpr::ENCODED_BLOCK_SIZE * blocks_wide * blocks_high);
    for coarse_y in 0..blocks_high {
        for coarse_x in 0..blocks_wide {
            for fine_y in 0..2 {
                for fine_x in 0..2 {
                    let offset =
                        8 * (2 * blocks_wide * (2 * coarse_y + fine_y) + 2 * coarse_x + fine_x);
                    data.extend_from_slice(&permute_dxt1_for_gamecube(
                        src_data[offset..offset + 8].try_into().unwrap(),
                    ));
                }
            }
        }
    }
    TextureBuf {
        logical_width: width,
        logical_height: height,
        physical_width,
        physical_height,
        data,
        _phantom_format: PhantomData,
    }
}

fn encode_dxt5_to_gx_tf_cmpr<F: TextureFormat>(
    width: usize,
    height: usize,
    src_data: &[u8],
) -> TextureBuf<F> {
    assert!(ptr::eq(F::as_dyn(), GxTfCmpr::as_dyn()));
    let physical_width = GxTfCmpr::physical_width(width);
    let physical_height = GxTfCmpr::physical_height(height);

    let blocks_wide = width / 8;
    let blocks_high = height / 8;
    let mut data = Vec::with_capacity(GxTfCmpr::ENCODED_BLOCK_SIZE * blocks_wide * blocks_high);
    for coarse_y in 0..blocks_high {
        for coarse_x in 0..blocks_wide {
            for fine_y in 0..2 {
                for fine_x in 0..2 {
                    let offset =
                        16 * (2 * blocks_wide * (2 * coarse_y + fine_y) + 2 * coarse_x + fine_x);
                    data.extend_from_slice(&permute_dxt1_for_gamecube(
                        src_data[offset + 8..offset + 16].try_into().unwrap(),
                    ));
                }
            }
        }
    }
    TextureBuf {
        logical_width: width,
        logical_height: height,
        physical_width,
        physical_height,
        data,
        _phantom_format: PhantomData,
    }
}

impl<F: TextureFormat> AnyTexture for TextureBuf<F>
where
    for<'a> TextureSlice<'a, F>: Into<AnyTextureSlice<'a>>,
{
    fn width(&self) -> usize {
        self.logical_width
    }

    fn height(&self) -> usize {
        self.logical_height
    }

    fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        self.get_texel(x, y)
    }

    fn as_slice(&self) -> AnyTextureSlice {
        <Self as Texture<F>>::as_slice(self).into()
    }

    fn dyn_format(&self) -> &'static dyn crate::DynTextureFormat {
        F::as_dyn()
    }
}

impl<F: TextureFormat> Texture<F> for TextureBuf<F> {
    fn width(&self) -> usize {
        self.logical_width
    }

    fn height(&self) -> usize {
        self.logical_height
    }

    fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        self.get_texel(x, y)
    }

    fn as_slice(&self) -> TextureSlice<F> {
        TextureSlice {
            logical_width: self.logical_width,
            logical_height: self.logical_height,
            physical_width: self.physical_width,
            physical_height: self.physical_height,
            data: &self.data,
            x0: 0,
            y0: 0,
            x1: self.logical_width,
            y1: self.logical_height,
            _phantom_format: PhantomData,
        }
    }

    fn data(&self) -> Option<&[u8]> {
        Some(&self.data)
    }
}

impl<F: TextureFormat> Texture<F> for &TextureBuf<F> {
    fn width(&self) -> usize {
        self.logical_width
    }

    fn height(&self) -> usize {
        self.logical_height
    }

    fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        (*self).get_texel(x, y)
    }

    fn as_slice(&self) -> TextureSlice<F> {
        TextureSlice {
            logical_width: self.logical_width,
            logical_height: self.logical_height,
            physical_width: self.physical_width,
            physical_height: self.physical_height,
            data: &self.data,
            x0: 0,
            y0: 0,
            x1: self.logical_width,
            y1: self.logical_height,
            _phantom_format: PhantomData,
        }
    }

    fn data(&self) -> Option<&[u8]> {
        Some(&self.data)
    }
}

impl<F> Clone for TextureBuf<F> {
    fn clone(&self) -> Self {
        Self {
            logical_width: self.logical_width,
            logical_height: self.logical_height,
            physical_width: self.physical_width,
            physical_height: self.physical_height,
            data: self.data.clone(),
            _phantom_format: PhantomData,
        }
    }
}
