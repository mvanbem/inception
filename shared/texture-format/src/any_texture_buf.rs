use alloc::vec::Vec;

use crate::{
    AnyTexture, Bgr8, Dxt1, Dxt5, DynTextureFormat, GxTfCmpr, GxTfRgba8, Rgb8, Rgba8, TextureBuf,
};

#[derive(Clone)]
pub enum AnyTextureBuf {
    Rgb8(TextureBuf<Rgb8>),
    Bgr8(TextureBuf<Bgr8>),
    Rgba8(TextureBuf<Rgba8>),
    Dxt1(TextureBuf<Dxt1>),
    Dxt5(TextureBuf<Dxt5>),
    GxTfRgba8(TextureBuf<GxTfRgba8>),
    GxTfCmpr(TextureBuf<GxTfCmpr>),
}

impl AnyTextureBuf {
    pub fn physical_width(&self) -> usize {
        match self {
            Self::Rgb8(x) => x.physical_width,
            Self::Bgr8(x) => x.physical_width,
            Self::Rgba8(x) => x.physical_width,
            Self::Dxt1(x) => x.physical_width,
            Self::Dxt5(x) => x.physical_width,
            Self::GxTfRgba8(x) => x.physical_width,
            Self::GxTfCmpr(x) => x.physical_width,
        }
    }

    pub fn physical_height(&self) -> usize {
        match self {
            Self::Rgb8(x) => x.physical_height,
            Self::Bgr8(x) => x.physical_height,
            Self::Rgba8(x) => x.physical_height,
            Self::Dxt1(x) => x.physical_height,
            Self::Dxt5(x) => x.physical_height,
            Self::GxTfRgba8(x) => x.physical_height,
            Self::GxTfCmpr(x) => x.physical_height,
        }
    }

    pub fn data(&self) -> &[u8] {
        match self {
            Self::Rgb8(x) => x.data(),
            Self::Bgr8(x) => x.data(),
            Self::Rgba8(x) => x.data(),
            Self::Dxt1(x) => x.data(),
            Self::Dxt5(x) => x.data(),
            Self::GxTfRgba8(x) => x.data(),
            Self::GxTfCmpr(x) => x.data(),
        }
    }

    pub fn into_data(self) -> Vec<u8> {
        match self {
            Self::Rgb8(x) => x.into_data(),
            Self::Bgr8(x) => x.into_data(),
            Self::Rgba8(x) => x.into_data(),
            Self::Dxt1(x) => x.into_data(),
            Self::Dxt5(x) => x.into_data(),
            Self::GxTfRgba8(x) => x.into_data(),
            Self::GxTfCmpr(x) => x.into_data(),
        }
    }

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

    pub fn format(&self) -> &'static dyn DynTextureFormat {
        self.as_any_texture().dyn_format()
    }
}

impl From<TextureBuf<Rgb8>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<Rgb8>) -> Self {
        Self::Rgb8(texture_buf)
    }
}

impl From<TextureBuf<Bgr8>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<Bgr8>) -> Self {
        Self::Bgr8(texture_buf)
    }
}

impl From<TextureBuf<Rgba8>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<Rgba8>) -> Self {
        Self::Rgba8(texture_buf)
    }
}

impl From<TextureBuf<Dxt1>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<Dxt1>) -> Self {
        Self::Dxt1(texture_buf)
    }
}

impl From<TextureBuf<Dxt5>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<Dxt5>) -> Self {
        Self::Dxt5(texture_buf)
    }
}

impl From<TextureBuf<GxTfRgba8>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<GxTfRgba8>) -> Self {
        Self::GxTfRgba8(texture_buf)
    }
}

impl From<TextureBuf<GxTfCmpr>> for AnyTextureBuf {
    fn from(texture_buf: TextureBuf<GxTfCmpr>) -> Self {
        Self::GxTfCmpr(texture_buf)
    }
}

impl AnyTexture for &AnyTextureBuf {
    fn width(&self) -> usize {
        self.as_any_texture().width()
    }

    fn height(&self) -> usize {
        self.as_any_texture().height()
    }

    fn get_texel(&self, x: usize, y: usize) -> [u8; 4] {
        self.as_any_texture().get_texel(x, y)
    }

    fn as_slice(&self) -> crate::AnyTextureSlice {
        self.as_any_texture().as_slice()
    }

    fn dyn_format(&self) -> &'static dyn DynTextureFormat {
        self.as_any_texture().dyn_format()
    }
}
