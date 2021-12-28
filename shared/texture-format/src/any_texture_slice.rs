use crate::{AnyTexture, Bgr8, Dxt1, Dxt5, GxTfCmpr, GxTfRgba8, Rgb8, Rgba8, TextureSlice};

#[derive(Clone, Copy)]
pub enum AnyTextureSlice<'a> {
    Rgb8(TextureSlice<'a, Rgb8>),
    Bgr8(TextureSlice<'a, Bgr8>),
    Rgba8(TextureSlice<'a, Rgba8>),
    Dxt1(TextureSlice<'a, Dxt1>),
    Dxt5(TextureSlice<'a, Dxt5>),
    GxTfRgba8(TextureSlice<'a, GxTfRgba8>),
    GxTfCmpr(TextureSlice<'a, GxTfCmpr>),
}

impl<'a> AnyTextureSlice<'a> {
    pub fn as_any_texture(&self) -> &(dyn AnyTexture + '_) {
        match self {
            Self::Rgb8(x) => x,
            Self::Bgr8(x) => x,
            Self::Rgba8(x) => x,
            Self::Dxt1(x) => x,
            Self::Dxt5(x) => x,
            Self::GxTfRgba8(x) => x,
            Self::GxTfCmpr(x) => x,
        }
    }
}

impl<'a> From<TextureSlice<'a, Rgb8>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, Rgb8>) -> Self {
        Self::Rgb8(texture_slice)
    }
}

impl<'a> From<TextureSlice<'a, Bgr8>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, Bgr8>) -> Self {
        Self::Bgr8(texture_slice)
    }
}

impl<'a> From<TextureSlice<'a, Rgba8>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, Rgba8>) -> Self {
        Self::Rgba8(texture_slice)
    }
}

impl<'a> From<TextureSlice<'a, Dxt1>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, Dxt1>) -> Self {
        Self::Dxt1(texture_slice)
    }
}

impl<'a> From<TextureSlice<'a, Dxt5>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, Dxt5>) -> Self {
        Self::Dxt5(texture_slice)
    }
}

impl<'a> From<TextureSlice<'a, GxTfRgba8>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, GxTfRgba8>) -> Self {
        Self::GxTfRgba8(texture_slice)
    }
}

impl<'a> From<TextureSlice<'a, GxTfCmpr>> for AnyTextureSlice<'a> {
    fn from(texture_slice: TextureSlice<'a, GxTfCmpr>) -> Self {
        Self::GxTfCmpr(texture_slice)
    }
}
