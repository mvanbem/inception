use std::rc::Rc;

use anyhow::{bail, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use texture_format::{TextureBuf, TextureFormat};

use crate::asset::{Asset, AssetLoader};
use crate::vpk::path::VpkPath;

pub struct Vtf {
    path: VpkPath,
    width: usize,
    height: usize,
    flags: u32,
    format: TextureFormat,
    face_count: usize,
    /// `mips[mip_level][face_index]`
    mips: Vec<Vec<TextureBuf>>,
}

#[derive(Clone, Copy)]
pub struct VtfFaceMip<'a> {
    pub face: usize,
    pub mip_level: usize,
    pub texture: &'a TextureBuf,
}

impl Vtf {
    fn bits_per_pixel_for_format(format: u32) -> Option<usize> {
        match format {
            13 => Some(4), // DXT1
            u32::MAX => Some(0),
            _ => {
                println!("unexpected image format: {}", format);
                None
            }
        }
    }

    pub fn path(&self) -> &VpkPath {
        &self.path
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn face_count(&self) -> usize {
        self.face_count
    }

    /// `mips()[mip_level][face_index]`
    pub fn mips(&self) -> &[Vec<TextureBuf>] {
        &self.mips
    }

    pub fn iter_face_mips(&self) -> impl Iterator<Item = VtfFaceMip> {
        FaceMipIter {
            vtf: self,
            face: 0,
            mip_level: 0,
        }
    }
}

impl Asset for Vtf {
    fn from_data(_loader: &AssetLoader, path: &VpkPath, data: Vec<u8>) -> Result<Rc<Self>> {
        let mut r = &*data;
        let signature = r.read_u32::<LittleEndian>()?;
        assert_eq!(signature, 0x00465456);
        let major_version = r.read_u32::<LittleEndian>()?;
        assert_eq!(major_version, 7);
        let minor_version = r.read_u32::<LittleEndian>()?;
        let header_size = r.read_u32::<LittleEndian>()?;
        let width = r.read_u16::<LittleEndian>()? as usize;
        let height = r.read_u16::<LittleEndian>()? as usize;
        let flags = r.read_u32::<LittleEndian>()?;
        let _frames = r.read_u16::<LittleEndian>()?;
        let first_frame = r.read_u16::<LittleEndian>()?;
        r = &r[4..];
        let _reflectivity_r = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let _reflectivity_g = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let _reflectivity_b = f32::from_bits(r.read_u32::<LittleEndian>()?);
        r = &r[4..];
        let _bump_map_scale = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let high_res_image_format = r.read_u32::<LittleEndian>()?;
        let mipmap_count = r.read_u8()? as usize;
        let low_res_image_format = r.read_u32::<LittleEndian>()?;
        let low_res_image_width = r.read_u8()?;
        let low_res_image_height = r.read_u8()?;

        if minor_version >= 2 {
            let depth = r.read_u16::<LittleEndian>()?;
            assert_eq!(depth, 1);
        }

        let face_count = if (flags & 0x4000) != 0 {
            assert_eq!(width, height);
            if first_frame != u16::MAX && minor_version < 5 {
                7
            } else {
                6
            }
        } else {
            1
        };

        let low_res_bpp = Self::bits_per_pixel_for_format(low_res_image_format);
        let (format, mips) = match low_res_bpp {
            Some(low_res_bpp) => {
                let high_res_offset = header_size as usize
                    + (low_res_image_width as usize * low_res_image_height as usize * low_res_bpp)
                        / 8;
                let high_res_data = &data[high_res_offset..];

                let format = match high_res_image_format {
                    3 => TextureFormat::Bgr8,
                    12 => TextureFormat::Bgra8,
                    13 => TextureFormat::Dxt1,
                    15 => TextureFormat::Dxt5,
                    16 => TextureFormat::Bgrx8,
                    24 => TextureFormat::Rgba16f,
                    _ => bail!(
                        "unexpected high res image format: {}",
                        high_res_image_format
                    ),
                };
                (
                    format,
                    build_mips(
                        format,
                        high_res_data,
                        mipmap_count,
                        face_count,
                        width,
                        height,
                    ),
                )
            }
            _ => bail!("unexpected low res image format: {}", low_res_image_format),
        };

        Ok(Rc::new(Vtf {
            path: path.to_owned(),
            width,
            height,
            flags,
            format,
            face_count,
            mips,
        }))
    }
}

fn build_mips(
    format: TextureFormat,
    mut data: &[u8],
    mipmap_count: usize,
    face_count: usize,
    width: usize,
    height: usize,
) -> Vec<Vec<TextureBuf>> {
    let mut mips = Vec::new();
    for index in 0..mipmap_count {
        let mip_level = mipmap_count - 1 - index;
        let mip_width = (width >> mip_level).max(1);
        let mip_height = (height >> mip_level).max(1);
        let size = format.metrics().encoded_size(mip_width, mip_height);

        let mut faces = Vec::new();
        for _ in 0..face_count {
            faces
                .push(TextureBuf::new(format, mip_width, mip_height, data[..size].to_vec()).into());
            data = &data[size..];
        }
        mips.push(faces);
    }
    mips.reverse();
    mips
}

struct FaceMipIter<'a> {
    vtf: &'a Vtf,
    // Valid until the iterator has ended, then forever out of range.
    face: usize,
    // Always valid.
    mip_level: usize,
}

impl<'a> Iterator for FaceMipIter<'a> {
    type Item = VtfFaceMip<'a>;

    fn next(&mut self) -> Option<VtfFaceMip<'a>> {
        if (self.face as usize) < self.vtf.face_count {
            // Prepare the result.
            let result = VtfFaceMip {
                face: self.face,
                mip_level: self.mip_level,
                texture: &self.vtf.mips[self.mip_level as usize][self.face as usize],
            };

            // Advance the counters.
            if ((self.mip_level + 1) as usize) < self.vtf.mips.len() {
                self.mip_level += 1;
            } else {
                self.mip_level = 0;
                self.face += 1;
            }

            return Some(result);
        }
        None
    }
}
