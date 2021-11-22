use std::rc::Rc;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

use crate::asset::{Asset, AssetLoader};
use crate::vpk::path::VpkPath;

pub struct Vtf {
    path: VpkPath,
    width: u32,
    height: u32,
    data: Option<ImageData>,
}

impl Vtf {
    fn bits_per_pixel_for_format(format: u32) -> Option<usize> {
        match format {
            13 => Some(4), // DXT1
            _ => {
                println!("unexpected image format: {}", format);
                None
            }
        }
    }

    pub fn path(&self) -> &VpkPath {
        &self.path
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> Option<&ImageData> {
        self.data.as_ref()
    }
}

impl Asset for Vtf {
    fn from_data(loader: &AssetLoader, path: &VpkPath, data: Vec<u8>) -> Result<Rc<Self>> {
        let mut r = &*data;
        let signature = r.read_u32::<LittleEndian>()?;
        assert_eq!(signature, 0x00465456);
        let major_version = r.read_u32::<LittleEndian>()?;
        let minor_version = r.read_u32::<LittleEndian>()?;
        let header_size = r.read_u32::<LittleEndian>()?;
        let width = r.read_u16::<LittleEndian>()?;
        let height = r.read_u16::<LittleEndian>()?;
        let flags = r.read_i32::<LittleEndian>()?;
        let frames = r.read_u16::<LittleEndian>()?;
        let first_frame = r.read_u16::<LittleEndian>()?;
        r = &r[4..];
        let reflectivity_r = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let reflectivity_g = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let reflectivity_b = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let reflectivity = [reflectivity_r, reflectivity_g, reflectivity_b];
        r = &r[4..];
        let bump_map_scale = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let high_res_image_format = r.read_u32::<LittleEndian>()?;
        let mipmap_count = r.read_u8()?;
        let low_res_image_format = r.read_u32::<LittleEndian>()?;
        let low_res_image_width = r.read_u8()?;
        let low_res_image_height = r.read_u8()?;

        let data = Self::bits_per_pixel_for_format(low_res_image_format).and_then(|low_res_bpp| {
            let high_res_offset = header_size as usize
                + (low_res_image_width as usize * low_res_image_height as usize * low_res_bpp) / 8;
            let high_res_data = &data[high_res_offset..];

            match high_res_image_format {
                3 => None, // BGR888
                13 => {
                    // DXT1
                    let mut data = high_res_data;
                    let mut mips = Vec::new();
                    for index in 0..mipmap_count {
                        let mip_level = mipmap_count - 1 - index;
                        let mut width = width as u32;
                        let mut height = height as u32;
                        for _ in 0..mip_level {
                            width = (width / 2).max(4);
                            height = (height / 2).max(4);
                        }

                        let size = width as usize * height as usize / 2;
                        mips.push(data[..size].to_vec());
                        data = &data[size..];
                    }
                    mips.reverse();
                    Some(ImageData {
                        format: ImageFormat::Dxt1,
                        mips,
                    })
                }
                15 => {
                    // DXT5
                    None
                }
                _ => {
                    println!("unexpected image format: {}", high_res_image_format);
                    None
                }
            }
        });

        Ok(Rc::new(Vtf {
            path: path.to_owned(),
            width: width as u32,
            height: height as u32,
            data,
        }))
    }
}

pub struct ImageData {
    pub format: ImageFormat,
    pub mips: Vec<Vec<u8>>,
}

pub enum ImageFormat {
    Dxt1,
}
