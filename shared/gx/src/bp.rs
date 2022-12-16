use modular_bitfield_msb::prelude::*;

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpGenModeReg {
    pub addr: B8,
    #[skip]
    unused: B4,
    pub zfreeze: bool,
    pub bump_tex_gen_count: B3,
    pub reject_en: B2, // this is an enum
    pub tev_stage_count_minus_one: B4,
    pub multisample: bool,
    #[skip]
    unused: B3,
    pub color_count: B2,   // 0..=2
    pub texture_count: B4, // 0..=8
}

impl BpGenModeReg {
    pub const ADDR: u8 = 0x00;
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexCoordRegA {
    pub addr: B8,
    #[skip]
    unused: B4,
    pub offset_for_points: bool,
    pub offset_for_lines: bool,
    pub s_cylindrical_wrapping: bool,
    pub s_range_bias: bool,
    pub s_scale_minus_one: u16,
}

impl_from!(BpTexCoordRegA, u32);

impl BpTexCoordRegA {
    const BASE_ADDR: u8 = 0x30;

    pub fn addr_for_texcoord(texcoord: u8) -> Option<u8> {
        if texcoord <= 7 {
            Some(Self::BASE_ADDR + (texcoord << 1))
        } else {
            None
        }
    }
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexCoordRegB {
    pub addr: B8,
    #[skip]
    unused: B6,
    pub t_cylindrical_wrapping: bool,
    pub t_range_bias: bool,
    pub t_scale_minus_one: u16,
}

impl_from!(BpTexCoordRegB, u32);

impl BpTexCoordRegB {
    const BASE_ADDR: u8 = 0x31;

    pub fn addr_for_texcoord(texcoord: u8) -> Option<u8> {
        if texcoord <= 7 {
            Some(Self::BASE_ADDR + (texcoord << 1))
        } else {
            None
        }
    }
}

pub trait BpInterleavedTexReg {
    const BASE_ADDR: u8;

    fn addr_for_image(image: u8) -> Option<u8> {
        if image <= 7 {
            Some(Self::BASE_ADDR + ((image & 0x04) << 3) | (image & 0x03))
        } else {
            None
        }
    }
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexModeRegA {
    pub addr: B8,
    #[skip]
    unused: B2,
    pub lod_clamp: bool,
    pub max_aniso: MaxAniso,
    #[skip]
    unused: B3,
    pub lod_bias: B7, // fixed point i2.5
    pub diag_lod: DiagLod,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub wrap_t: Wrap,
    pub wrap_s: Wrap,
}

impl_from!(BpTexModeRegA, u32);

impl BpInterleavedTexReg for BpTexModeRegA {
    const BASE_ADDR: u8 = 0x80;
}

#[derive(BitfieldSpecifier)]
#[bits = 2]
pub enum MaxAniso {
    _1,
    /// NOTE: Requires `diag_lod: Edge`.
    _2,
    /// NOTE: Requires `diag_lod: Edge`.
    _4,
}

#[derive(BitfieldSpecifier)]
pub enum DiagLod {
    EdgeLod,
    DiagonalLod,
}

#[derive(BitfieldSpecifier)]
#[bits = 3]
pub enum MinFilter {
    Nearest = 0,
    NearestMipNearest = 1,
    NearestMipLinear = 2,
    Linear = 4,
    LinearMipNearest = 5,
    LinearMipLinear = 6,
}

#[derive(BitfieldSpecifier)]
pub enum MagFilter {
    Nearest,
    Linear,
}

#[derive(BitfieldSpecifier)]
#[bits = 2]
pub enum Wrap {
    Clamp,
    /// NOTE: Requires a power of two texture size in this dimension.
    Repeat,
    /// NOTE: Requires a power of two texture size in this dimension.
    Mirror,
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexModeRegB {
    pub addr: B8,
    #[skip]
    unused: B8,
    pub max_lod: u8, // fixed point u4.4
    pub min_lod: u8, // fixed point u4.4
}

impl_from!(BpTexModeRegB, u32);

impl BpInterleavedTexReg for BpTexModeRegB {
    const BASE_ADDR: u8 = 0x84;
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexImageRegA {
    pub addr: B8,
    pub format: TextureFormat,
    pub height_minus_one: B10,
    pub width_minus_one: B10,
}

impl_from!(BpTexImageRegA, u32);

impl BpInterleavedTexReg for BpTexImageRegA {
    const BASE_ADDR: u8 = 0x88;
}

#[derive(BitfieldSpecifier)]
#[bits = 4]
pub enum TextureFormat {
    I4 = 0,
    I8 = 1,
    Ia4 = 2,
    Ia8 = 3,
    Rgb565 = 4,
    Rgb5a3 = 5,
    Rgba8 = 6,
    C4 = 8,
    C8 = 9,
    C14x2 = 10,
    Cmp = 14,
}

/// Defines the TMEM behavior for even LODs and whether the image is cached or preloaded.
#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexImageRegB {
    pub addr: B8,
    #[skip]
    unused: B2,
    pub image_type: ImageType,
    pub cache_height: CacheSize,
    pub cache_width: CacheSize,
    /// TMEM address >> 5.
    pub tmem_offset: B15,
}

impl_from!(BpTexImageRegB, u32);

impl BpInterleavedTexReg for BpTexImageRegB {
    const BASE_ADDR: u8 = 0x8c;
}

#[derive(BitfieldSpecifier)]
pub enum ImageType {
    Cached,
    Preloaded,
}

/// Defines the TMEM behavior for odd LODs.
#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexImageRegC {
    pub addr: B8,
    #[skip]
    unused: B3,
    pub cache_height: CacheSize,
    pub cache_width: CacheSize,
    /// TMEM address >> 5.
    pub tmem_offset: B15,
}

impl_from!(BpTexImageRegC, u32);

impl BpInterleavedTexReg for BpTexImageRegC {
    const BASE_ADDR: u8 = 0x90;
}

#[derive(BitfieldSpecifier)]
#[bits = 3]
pub enum CacheSize {
    _32KB = 3,
    _128KB = 4,
    _512KB = 5,
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct BpTexImageRegD {
    pub addr: B8,
    /// Main memory physical address >> 5.
    pub address: B24,
}

impl_from!(BpTexImageRegD, u32);

impl BpInterleavedTexReg for BpTexImageRegD {
    const BASE_ADDR: u8 = 0x94;
}
