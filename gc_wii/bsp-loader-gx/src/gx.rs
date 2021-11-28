#![allow(dead_code)]

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevColorIn {
    PrevColor = 0,
    PrevAlpha = 1,
    Reg0Color = 2,
    Reg0Alpha = 3,
    Reg1Color = 4,
    Reg1Alpha = 5,
    Reg2Color = 6,
    Reg2Alpha = 7,
    TexColor = 8,
    TexAlpha = 9,
    RasColor = 10,
    RasAlpha = 11,
    Constant1 = 12,
    Constant1_2 = 13,
    Konst = 14,
    Constant0 = 15,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevAlphaIn {
    PrevAlpha = 0,
    Reg0Alpha = 1,
    Reg1Alpha = 2,
    Reg2Alpha = 3,
    TexAlpha = 4,
    RasAlpha = 5,
    Konst = 6,
    Constant0 = 7,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevOp {
    Add = 0,
    Sub = 1,
    CompR8Gt = 8,
    CompR8Eq = 9,
    CompGr16Gt = 10,
    CompGr16Eq = 11,
    CompBgr24Gt = 12,
    CompBgr24Eq = 13,
    CompRgb8OrA8Gt = 14,
    CompRgb8OrA8Eq = 15,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevBias {
    Zero = 0,
    AddHalf = 1,
    SubHalf = 2,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevScale {
    K1 = 0,
    K2 = 1,
    K4 = 2,
    K1_2 = 3,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevReg {
    Prev = 0,
    Reg0 = 1,
    Reg1 = 2,
    Reg2 = 3,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevColorKonst {
    Constant1 = 0x00,
    Constant7_8 = 0x01,
    Constant3_4 = 0x02,
    Constant5_8 = 0x03,
    Constant1_2 = 0x04,
    Constant3_8 = 0x05,
    Constant1_4 = 0x06,
    Constant1_8 = 0x07,
    K0Rgb = 0x0c,
    K1Rgb = 0x0d,
    K2Rgb = 0x0e,
    K3Rgb = 0x0f,
    K0R = 0x10,
    K1R = 0x11,
    K2R = 0x12,
    K3R = 0x13,
    K0G = 0x14,
    K1G = 0x15,
    K2G = 0x16,
    K3G = 0x17,
    K0B = 0x18,
    K1B = 0x19,
    K2B = 0x1a,
    K3B = 0x1b,
    K0A = 0x1c,
    K1A = 0x1d,
    K2A = 0x1e,
    K3A = 0x1f,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevAlphaKonst {
    Constant1 = 0x00,
    Constant7_8 = 0x01,
    Constant3_4 = 0x02,
    Constant5_8 = 0x03,
    Constant1_2 = 0x04,
    Constant3_8 = 0x05,
    Constant1_4 = 0x06,
    Constant1_8 = 0x07,
    K0R = 0x10,
    K1R = 0x11,
    K2R = 0x12,
    K3R = 0x13,
    K0G = 0x14,
    K1G = 0x15,
    K2G = 0x16,
    K3G = 0x17,
    K0B = 0x18,
    K1B = 0x19,
    K2B = 0x1a,
    K3B = 0x1b,
    K0A = 0x1c,
    K1A = 0x1d,
    K2A = 0x1e,
    K3A = 0x1f,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevTexCoord {
    TexCoord0 = 0x00,
    TexCoord1 = 0x01,
    TexCoord2 = 0x02,
    TexCoord3 = 0x03,
    TexCoord4 = 0x04,
    TexCoord5 = 0x05,
    TexCoord6 = 0x06,
    TexCoord7 = 0x07,
    Null = 0xff,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
)]
pub struct TevTexMap(u32);

impl TevTexMap {
    pub const TEXMAP0: Self = Self(0);
    pub const TEXMAP1: Self = Self(1);
    pub const TEXMAP2: Self = Self(2);
    pub const TEXMAP3: Self = Self(3);
    pub const TEXMAP4: Self = Self(4);
    pub const TEXMAP5: Self = Self(5);
    pub const TEXMAP6: Self = Self(6);
    pub const TEXMAP7: Self = Self(7);
    pub const NULL: Self = Self(0xff);
    pub const DISABLE_FLAG: Self = Self(0x100);

    pub fn from_u32(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TevChannel {
    Color0 = 0,
    Color1 = 1,
    Alpha0 = 2,
    Alpha1 = 3,
    Color0A0 = 4,
    Color1A1 = 5,
    Zero = 6,
    AlphaBump = 7,
    AlphaBumpN = 8,
    Null = 0xff,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TexGenType {
    Mtx3x4 = 0,
    Mtx2x4 = 1,
    Bump0 = 2,
    Bump1 = 3,
    Bump2 = 4,
    Bump3 = 5,
    Bump4 = 6,
    Bump5 = 7,
    Bump6 = 8,
    Bump7 = 9,
    SrTg = 10,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TexGenSrc {
    Position = 0,
    Normal = 1,
    Binormal = 2,
    Tangent = 3,
    Tex0 = 4,
    Tex1 = 5,
    Tex2 = 6,
    Tex3 = 7,
    Tex4 = 8,
    Tex5 = 9,
    Tex6 = 10,
    Tex7 = 11,
    TexCoord0 = 12,
    TexCoord1 = 13,
    TexCoord2 = 14,
    TexCoord3 = 15,
    TexCoord4 = 16,
    TexCoord5 = 17,
    TexCoord6 = 18,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexMtxIndex(u32);

impl TexMtxIndex {
    pub const IDENTITY: Self = Self(60);
    pub const TEXMTX0: Self = Self(30);
    pub const TEXMTX1: Self = Self(33);
    pub const TEXMTX2: Self = Self(36);
    pub const TEXMTX3: Self = Self(39);
    pub const TEXMTX4: Self = Self(42);
    pub const TEXMTX5: Self = Self(45);
    pub const TEXMTX6: Self = Self(48);
    pub const TEXMTX7: Self = Self(51);
    pub const TEXMTX8: Self = Self(54);
    pub const TEXMTX9: Self = Self(57);

    pub fn from_u32(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostTransformTexMtxIndex(u32);

impl PostTransformTexMtxIndex {
    pub const IDENTITY: Self = Self(125);
    pub const DTTMTX0: Self = Self(64);
    pub const DTTMTX1: Self = Self(67);
    pub const DTTMTX2: Self = Self(70);
    pub const DTTMTX3: Self = Self(73);
    pub const DTTMTX4: Self = Self(76);
    pub const DTTMTX5: Self = Self(79);
    pub const DTTMTX6: Self = Self(82);
    pub const DTTMTX7: Self = Self(85);
    pub const DTTMTX8: Self = Self(88);
    pub const DTTMTX9: Self = Self(91);

    pub fn from_u32(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}
