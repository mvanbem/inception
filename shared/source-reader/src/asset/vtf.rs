use std::rc::Rc;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use texture_atlas::RgbU8Image;

use crate::asset::{Asset, AssetLoader};
use crate::vpk::path::VpkPath;

pub struct Vtf {
    path: VpkPath,
    width: u32,
    height: u32,
    flags: u32,
    data: Option<ImageData>,
}

impl Vtf {
    pub fn new(path: VpkPath, image: &RgbU8Image) -> Self {
        Self {
            path,
            width: image.width() as u32,
            height: image.height() as u32,
            flags: 0,
            data: Some(ImageData {
                format: ImageFormat::Rgb8,
                layer_count: 1,
                mips: vec![vec![image.data().to_vec()]],
            }),
        }
    }

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

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn data(&self) -> Option<&ImageData> {
        self.data.as_ref()
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
        let width = r.read_u16::<LittleEndian>()?;
        let height = r.read_u16::<LittleEndian>()?;
        let flags = r.read_i32::<LittleEndian>()?;
        let _frames = r.read_u16::<LittleEndian>()?;
        let first_frame = r.read_u16::<LittleEndian>()?;
        r = &r[4..];
        let _reflectivity_r = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let _reflectivity_g = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let _reflectivity_b = f32::from_bits(r.read_u32::<LittleEndian>()?);
        r = &r[4..];
        let _bump_map_scale = f32::from_bits(r.read_u32::<LittleEndian>()?);
        let high_res_image_format = r.read_u32::<LittleEndian>()?;
        let mipmap_count = r.read_u8()?;
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
                    3 => {
                        // BGR888
                        let mut data = high_res_data;
                        let mut mips = Vec::new();
                        for index in 0..mipmap_count {
                            let mip_level = mipmap_count - 1 - index;
                            let mut width = width as u32;
                            let mut height = height as u32;
                            for _ in 0..mip_level {
                                width = (width / 2).max(1);
                                height = (height / 2).max(1);
                            }

                            let mut layers = Vec::new();
                            for _ in 0..layer_count {
                                let pixel_count = width as usize * height as usize;
                                let mut pixels = Vec::with_capacity(pixel_count * 3);
                                for _ in 0..pixel_count {
                                    let b = data.read_u8()?;
                                    let g = data.read_u8()?;
                                    let r = data.read_u8()?;
                                    pixels.extend_from_slice(&[r, g, b]);
                                }
                                layers.push(pixels);
                            }
                            mips.push(layers);
                        }
                        mips.reverse();
                        Some(ImageData {
                            format: ImageFormat::Rgb8,
                            layer_count,
                            mips,
                        })
                    }
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

                            let mut layers = Vec::new();
                            for _ in 0..layer_count {
                                let size = width as usize * height as usize / 2;
                                layers.push(data[..size].to_vec());
                                data = &data[size..];
                            }
                            mips.push(layers);
                        }
                        mips.reverse();
                        Some(ImageData {
                            format: ImageFormat::Dxt1,
                            layer_count,
                            mips,
                        })
                    }
                    15 => {
                        // DXT5
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

                            let mut layers = Vec::new();
                            for _ in 0..layer_count {
                                let size = width as usize * height as usize;
                                let mut pixels =
                                    Vec::with_capacity(width as usize * height as usize * 4);
                                let coarse_width = width as usize / 4;
                                for y in 0..height as usize {
                                    for x in 0..width as usize {
                                        let coarse_x = x / 4;
                                        let coarse_y = y / 4;
                                        let block_offset =
                                            16 * (coarse_width * coarse_y + coarse_x);
                                        let alpha_block = &data[block_offset..block_offset + 8];
                                        let mut color_block =
                                            &data[block_offset + 8..block_offset + 16];
                                        let fine_x = x % 4;
                                        let fine_y = y % 4;

                                        let a0 = alpha_block[0];
                                        let a1 = alpha_block[1];
                                        let alphas = if a0 > a1 {
                                            [
                                                a0,
                                                a1,
                                                ((6 * a0 as u16 + 1 * a1 as u16) / 7) as u8,
                                                ((5 * a0 as u16 + 2 * a1 as u16) / 7) as u8,
                                                ((4 * a0 as u16 + 3 * a1 as u16) / 7) as u8,
                                                ((3 * a0 as u16 + 4 * a1 as u16) / 7) as u8,
                                                ((2 * a0 as u16 + 5 * a1 as u16) / 7) as u8,
                                                ((1 * a0 as u16 + 6 * a1 as u16) / 7) as u8,
                                            ]
                                        } else {
                                            [
                                                a0,
                                                a1,
                                                ((4 * a0 as u16 + 1 * a1 as u16) / 5) as u8,
                                                ((3 * a0 as u16 + 2 * a1 as u16) / 5) as u8,
                                                ((2 * a0 as u16 + 3 * a1 as u16) / 5) as u8,
                                                ((1 * a0 as u16 + 4 * a1 as u16) / 5) as u8,
                                                0,
                                                255,
                                            ]
                                        };
                                        let alpha_bit = 3 * (4 * fine_y + fine_x);
                                        let alpha_bits = alpha_block[2] as u64
                                            | ((alpha_block[3] as u64) << 8)
                                            | ((alpha_block[4] as u64) << 16)
                                            | ((alpha_block[5] as u64) << 24)
                                            | ((alpha_block[6] as u64) << 32)
                                            | ((alpha_block[7] as u64) << 40);
                                        let a = alphas[((alpha_bits >> alpha_bit) & 7) as usize];

                                        let [r0, g0, b0] = decode_rgb565(
                                            color_block.read_u16::<LittleEndian>().unwrap(),
                                        );
                                        let [r1, g1, b1] = decode_rgb565(
                                            color_block.read_u16::<LittleEndian>().unwrap(),
                                        );
                                        let colors = [
                                            [r0, g0, b0],
                                            [r1, g1, b1],
                                            [
                                                ((2 * r0 as u16 + 1 * r1 as u16) / 3) as u8,
                                                ((2 * g0 as u16 + 1 * g1 as u16) / 3) as u8,
                                                ((2 * b0 as u16 + 1 * b1 as u16) / 3) as u8,
                                            ],
                                            [
                                                ((1 * r0 as u16 + 2 * r1 as u16) / 3) as u8,
                                                ((1 * g0 as u16 + 2 * g1 as u16) / 3) as u8,
                                                ((1 * b0 as u16 + 2 * b1 as u16) / 3) as u8,
                                            ],
                                        ];
                                        let color_bit = 2 * (4 * fine_y + fine_x);
                                        let color_bits =
                                            color_block.read_u32::<LittleEndian>().unwrap();
                                        let [r, g, b] =
                                            colors[((color_bits >> color_bit) & 3) as usize];

                                        pixels.extend_from_slice(&[r, g, b, a]);
                                    }
                                }
                                layers.push(pixels);
                                data = &data[size..];
                            }
                            mips.push(layers);
                        }
                        mips.reverse();
                        Some(ImageData {
                            format: ImageFormat::Rgba8,
                            layer_count,
                            mips,
                        })
                    }
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
            width: width as u32,
            height: height as u32,
            flags: flags as u32,
            data,
        }))
    }
}

fn decode_rgb565(encoded: u16) -> [u8; 3] {
    let extend5 = |x| (x << 3) | (x >> 2);
    let extend6 = |x| (x << 2) | (x >> 4);
    [
        extend5(((encoded >> 11) & 0x1f) as u8),
        extend6(((encoded >> 5) & 0x3f) as u8),
        extend5((encoded & 0x1f) as u8),
    ]
}

pub struct ImageData {
    pub format: ImageFormat,
    pub layer_count: usize,
    /// `mips[mip_level][layer_index][byte]`
    pub mips: Vec<Vec<Vec<u8>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Dxt1,
    Rgb8,
    Rgba8,
}
