use core::marker::PhantomData;
use core::ops::Range;

use crate::{AnyTexture, AnyTextureSlice, Texture, TextureFormat};

pub struct TextureSlice<'a, F> {
    pub(crate) logical_width: usize,
    pub(crate) logical_height: usize,
    pub(crate) physical_width: usize,
    pub(crate) physical_height: usize,
    pub(crate) data: &'a [u8],
    pub(crate) x0: usize,
    pub(crate) y0: usize,
    pub(crate) x1: usize,
    pub(crate) y1: usize,
    pub(crate) _phantom_format: PhantomData<*const F>,
}

impl<'a, F: TextureFormat> TextureSlice<'a, F> {
    pub fn x_range(self) -> Range<usize> {
        self.x0..self.x1
    }

    pub fn y_range(self) -> Range<usize> {
        self.y0..self.y1
    }

    pub fn width(self) -> usize {
        self.x1 - self.x0
    }

    pub fn height(self) -> usize {
        self.y1 - self.y0
    }

    pub fn get_texel(self, x: usize, y: usize) -> [u8; 4] {
        assert!(self.x_range().contains(&x) && self.y_range().contains(&y));
        F::get_texel(
            self.physical_width,
            self.physical_height,
            self.data,
            self.x0 + x,
            self.y0 + y,
        )
    }
}

impl<'a, F> Clone for TextureSlice<'a, F> {
    fn clone(&self) -> Self {
        Self {
            logical_width: self.logical_width,
            logical_height: self.logical_height,
            physical_width: self.physical_width,
            physical_height: self.physical_height,
            data: self.data,
            x0: self.x0,
            y0: self.y0,
            x1: self.x1,
            y1: self.y1,
            _phantom_format: PhantomData,
        }
    }
}

impl<'a, F> Copy for TextureSlice<'a, F> {}

impl<'a, F: TextureFormat> AnyTexture for TextureSlice<'a, F>
where
    TextureSlice<'a, F>: Into<AnyTextureSlice<'a>>,
{
    fn width(&self) -> usize {
        Self::width(*self)
    }

    fn height(&self) -> usize {
        Self::height(*self)
    }

    fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        Self::get_texel(*self, x, y)
    }

    fn as_slice(&self) -> AnyTextureSlice<'a> {
        <TextureSlice<F> as Into<AnyTextureSlice>>::into(*self)
    }

    fn dyn_format(&self) -> &'static dyn crate::DynTextureFormat {
        F::as_dyn()
    }
}

impl<'a, F: TextureFormat> Texture<F> for TextureSlice<'a, F> {
    fn width(&self) -> usize {
        Self::width(*self)
    }

    fn height(&self) -> usize {
        Self::height(*self)
    }

    fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        Self::get_texel(*self, x, y)
    }

    fn as_slice(&self) -> TextureSlice<F> {
        *self
    }
}
