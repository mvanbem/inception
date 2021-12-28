use std::borrow::Cow;

use anyhow::Result;
use glium::texture::{
    ClientFormat, CompressedMipmapsOption, CompressedSrgbFormat, CompressedSrgbTexture2d,
    CompressedTexture2d, MipmapsOption, RawImage2d, SrgbFormat, SrgbTexture2d,
};
use glium::{Display, Rect, Texture2d};
use source_reader::asset::vtf::Vtf;
use texture_format::{AnyTexture, AnyTextureBuf, TextureBuf, TextureFormat};

pub enum AnyTexture2d {
    Texture2d(Texture2d),
    SrgbTexture2d(SrgbTexture2d),
    CompressedTexture2d(CompressedTexture2d),
    CompressedSrgbTexture2d(CompressedSrgbTexture2d),
}

impl AnyTexture2d {
    pub fn width(&self) -> u32 {
        match self {
            Self::Texture2d(x) => x.width(),
            Self::SrgbTexture2d(x) => x.width(),
            Self::CompressedTexture2d(x) => x.width(),
            Self::CompressedSrgbTexture2d(x) => x.width(),
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Texture2d(x) => x.height(),
            Self::SrgbTexture2d(x) => x.height(),
            Self::CompressedTexture2d(x) => x.height(),
            Self::CompressedSrgbTexture2d(x) => x.height(),
        }
    }
}

impl From<Texture2d> for AnyTexture2d {
    fn from(texture: Texture2d) -> Self {
        Self::Texture2d(texture)
    }
}

impl From<SrgbTexture2d> for AnyTexture2d {
    fn from(texture: SrgbTexture2d) -> Self {
        Self::SrgbTexture2d(texture)
    }
}

impl From<CompressedTexture2d> for AnyTexture2d {
    fn from(texture: CompressedTexture2d) -> Self {
        Self::CompressedTexture2d(texture)
    }
}

impl From<CompressedSrgbTexture2d> for AnyTexture2d {
    fn from(texture: CompressedSrgbTexture2d) -> Self {
        Self::CompressedSrgbTexture2d(texture)
    }
}

pub trait CreateGliumTexture {
    type Texture: Into<AnyTexture2d>;

    fn create_texture(
        display: &Display,
        width: u32,
        height: u32,
        mip_count: u32,
    ) -> Result<Self::Texture>;

    fn write_mip(texture: &Self::Texture, mip_level: u32, src: &AnyTextureBuf) -> Result<()>;
}

pub fn create_texture<C: CreateGliumTexture>(display: &Display, src: &Vtf) -> Result<AnyTexture2d> {
    let dst = C::create_texture(
        display,
        src.width() as u32,
        src.height() as u32,
        src.data().unwrap().mips.len() as u32,
    )?;

    for face_mip in src.iter_face_mips() {
        assert_eq!(face_mip.face, 0);
        C::write_mip(&dst, face_mip.mip_level as u32, face_mip.texture)?;
    }

    Ok(dst.into())
}

pub fn create_texture_encoded<C: CreateGliumTexture, F: TextureFormat>(
    display: &Display,
    src: &Vtf,
) -> Result<AnyTexture2d>
where
    TextureBuf<F>: Into<AnyTextureBuf>,
{
    let dst = C::create_texture(
        display,
        src.width() as u32,
        src.height() as u32,
        src.data().unwrap().mips.len() as u32,
    )?;

    for face_mip in src.iter_face_mips() {
        assert_eq!(face_mip.face, 0);
        C::write_mip(
            &dst,
            face_mip.mip_level as u32,
            &TextureBuf::<F>::encode_any(face_mip.texture).into(),
        )?;
    }

    Ok(dst.into())
}

pub struct CreateSrgbTexture2dRgba8;

impl CreateGliumTexture for CreateSrgbTexture2dRgba8 {
    type Texture = SrgbTexture2d;

    fn create_texture(
        display: &Display,
        width: u32,
        height: u32,
        mip_count: u32,
    ) -> Result<Self::Texture> {
        Ok(Self::Texture::empty_with_format(
            display,
            SrgbFormat::U8U8U8U8,
            MipmapsOption::EmptyMipmapsMax(mip_count - 1),
            width,
            height,
        )?)
    }

    fn write_mip(dst: &Self::Texture, mip_level: u32, src: &AnyTextureBuf) -> Result<()> {
        let dst_mip = dst.mipmap(mip_level).unwrap();
        assert_eq!(src.width(), dst_mip.width() as usize);
        assert_eq!(src.height(), dst_mip.height() as usize);

        dst_mip.write(
            Rect {
                left: 0,
                bottom: 0,
                width: src.width() as u32,
                height: src.height() as u32,
            },
            RawImage2d {
                data: Cow::Borrowed(src.data()),
                width: src.width() as u32,
                height: src.height() as u32,
                format: ClientFormat::U8U8U8U8,
            },
        );
        Ok(())
    }
}

pub struct CreateCompressedSrgbTexture2dDxt1;

impl CreateGliumTexture for CreateCompressedSrgbTexture2dDxt1 {
    type Texture = CompressedSrgbTexture2d;

    fn create_texture(
        display: &Display,
        width: u32,
        height: u32,
        mip_count: u32,
    ) -> Result<Self::Texture> {
        Ok(Self::Texture::empty_with_format(
            display,
            CompressedSrgbFormat::S3tcDxt1Alpha,
            CompressedMipmapsOption::EmptyMipmapsMax(mip_count - 1),
            width,
            height,
        )?)
    }

    fn write_mip(dst: &Self::Texture, mip_level: u32, src: &AnyTextureBuf) -> Result<()> {
        let dst_mip = dst.mipmap(mip_level).unwrap();
        assert_eq!(src.width(), dst_mip.width() as usize);
        assert_eq!(src.height(), dst_mip.height() as usize);

        dst_mip
            .write_compressed_data(
                Rect {
                    left: 0,
                    bottom: 0,
                    width: src.width() as u32,
                    height: src.height() as u32,
                },
                src.data(),
                src.width() as u32,
                src.height() as u32,
                CompressedSrgbFormat::S3tcDxt1Alpha,
            )
            .unwrap();
        Ok(())
    }
}

pub struct CreateCompressedSrgbTexture2dDxt5;

impl CreateGliumTexture for CreateCompressedSrgbTexture2dDxt5 {
    type Texture = CompressedSrgbTexture2d;

    fn create_texture(
        display: &Display,
        width: u32,
        height: u32,
        mip_count: u32,
    ) -> Result<Self::Texture> {
        Ok(Self::Texture::empty_with_format(
            display,
            CompressedSrgbFormat::S3tcDxt5Alpha,
            CompressedMipmapsOption::EmptyMipmapsMax(mip_count - 1),
            width,
            height,
        )?)
    }

    fn write_mip(dst: &Self::Texture, mip_level: u32, src: &AnyTextureBuf) -> Result<()> {
        let dst_mip = dst.mipmap(mip_level).unwrap();
        assert_eq!(src.width(), dst_mip.width() as usize);
        assert_eq!(src.height(), dst_mip.height() as usize);

        dst_mip
            .write_compressed_data(
                Rect {
                    left: 0,
                    bottom: 0,
                    width: src.width() as u32,
                    height: src.height() as u32,
                },
                src.data(),
                src.width() as u32,
                src.height() as u32,
                CompressedSrgbFormat::S3tcDxt5Alpha,
            )
            .unwrap();
        Ok(())
    }
}
