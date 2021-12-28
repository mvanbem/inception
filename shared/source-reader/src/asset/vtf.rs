use std::rc::Rc;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use texture_format::{
    AnyTextureBuf, Bgr8, Dxt1, Dxt5, DynTextureFormat, TextureBuf, TextureFormat, TextureFormatExt,
};

use crate::asset::{Asset, AssetLoader};
use crate::vpk::path::VpkPath;

pub struct Vtf {
    path: VpkPath,
    width: usize,
    height: usize,
    flags: u32,
    data: Option<ImageData>,
}

pub struct VtfFaceMip<'a> {
    pub face: usize,
    pub mip_level: usize,
    pub texture: &'a AnyTextureBuf,
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

    pub fn data(&self) -> Option<&ImageData> {
        self.data.as_ref()
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

        let layer_count = if (flags & 0x4000) != 0 {
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
        let data = match low_res_bpp {
            Some(low_res_bpp) => {
                let high_res_offset = header_size as usize
                    + (low_res_image_width as usize * low_res_image_height as usize * low_res_bpp)
                        / 8;
                let high_res_data = &data[high_res_offset..];

                match high_res_image_format {
                    3 => Some(build_image_data::<Bgr8>(
                        high_res_data,
                        mipmap_count,
                        layer_count,
                        width,
                        height,
                    )),
                    13 => Some(build_image_data::<Dxt1>(
                        high_res_data,
                        mipmap_count,
                        layer_count,
                        width,
                        height,
                    )),
                    15 => Some(build_image_data::<Dxt5>(
                        high_res_data,
                        mipmap_count,
                        layer_count,
                        width,
                        height,
                    )),
                    _ => {
                        println!("unexpected image format: {}", high_res_image_format);
                        None
                    }
                }
            }
            None => None,
        };

        Ok(Rc::new(Vtf {
            path: path.to_owned(),
            width,
            height,
            flags,
            data,
        }))
    }
}

fn build_image_data<F: TextureFormat>(
    mut data: &[u8],
    mipmap_count: usize,
    layer_count: usize,
    width: usize,
    height: usize,
) -> ImageData
where
    TextureBuf<F>: Into<AnyTextureBuf>,
{
    let mut mips = Vec::new();
    for index in 0..mipmap_count {
        let mip_level = mipmap_count - 1 - index;
        let mip_width = (width >> mip_level).max(1);
        let mip_height = (height >> mip_level).max(1);
        let size = F::encoded_size(mip_width, mip_height);

        let mut layers = Vec::new();
        for _ in 0..layer_count {
            layers.push(TextureBuf::<F>::new(mip_width, mip_height, data[..size].to_vec()).into());
            data = &data[size..];
        }
        mips.push(layers);
    }
    mips.reverse();
    ImageData {
        format: F::as_dyn(),
        layer_count,
        mips,
    }
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
        if let Some(image_data) = self.vtf.data.as_ref() {
            if (self.face as usize) < image_data.layer_count {
                // Prepare the result.
                let data = &image_data.mips[self.mip_level as usize][self.face as usize];
                let result = VtfFaceMip {
                    face: self.face,
                    mip_level: self.mip_level,
                    texture: &data,
                };

                // Advance the counters.
                if ((self.mip_level + 1) as usize) < image_data.mips.len() {
                    self.mip_level += 1;
                } else {
                    self.mip_level = 0;
                    self.face += 1;
                }

                return Some(result);
            }
        }
        None
    }
}

pub struct ImageData {
    pub format: &'static dyn DynTextureFormat,
    pub layer_count: usize,
    /// `mips[mip_level][layer_index]`
    pub mips: Vec<Vec<AnyTextureBuf>>,
}
