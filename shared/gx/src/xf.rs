use modular_bitfield_msb::prelude::*;

use crate::common::{MatrixRegA, MatrixRegB, PostTransformMatrix};

pub trait XfReg {
    type T: Into<u32>;

    fn addr(&self) -> u16;
}

// TODO: 0x1012 (dual texture transform): Does this affect performance? Is it related to the texgen
// normalize and post-transform matrix multiply?

pub struct XfMatrixRegA;

impl XfReg for XfMatrixRegA {
    type T = MatrixRegA;

    fn addr(&self) -> u16 {
        0x1018
    }
}

pub struct XfMatrixRegB;

impl XfReg for XfMatrixRegB {
    type T = MatrixRegB;

    fn addr(&self) -> u16 {
        0x1019
    }
}

pub struct TexCoordGenCountReg;

impl XfReg for TexCoordGenCountReg {
    type T = u32;

    fn addr(&self) -> u16 {
        0x103f
    }
}

pub struct XfTexCoordGenRegA {
    index: u8,
}

impl XfTexCoordGenRegA {
    pub const TEX0: Self = Self { index: 0 };
    pub const TEX1: Self = Self { index: 1 };
    pub const TEX2: Self = Self { index: 2 };
    pub const TEX3: Self = Self { index: 3 };
    pub const TEX4: Self = Self { index: 4 };
    pub const TEX5: Self = Self { index: 5 };
    pub const TEX6: Self = Self { index: 6 };
    pub const TEX7: Self = Self { index: 7 };

    pub fn new(index: u8) -> Option<Self> {
        if index <= 7 {
            Some(Self { index })
        } else {
            None
        }
    }
}

impl XfReg for XfTexCoordGenRegA {
    type T = TexCoordGenRegA;

    fn addr(&self) -> u16 {
        0x1040 | self.index as u16
    }
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct TexCoordGenRegA {
    #[skip]
    unused: B14,
    /// Light index used as input for `mode: EmbossMap`.
    pub emboss_light: B3,
    /// Texture coordinate index used as input for `mode: EmbossMap`.
    pub emboss_source: B3,
    #[skip]
    unused: B1,
    /// Vertex component to use as source for `mode: Regular`. (and both SrTg modes?)
    pub source_row: TexCoordGenSource,
    #[skip]
    unused: B1,
    pub mode: TexCoordGenMode,
    #[skip]
    unused: B1,
    pub input_form: TexCoordGenInputForm,
    pub projection: TexCoordProjection,
    #[skip]
    unused: B1,
}

impl_from!(TexCoordGenRegA, u32);

#[derive(BitfieldSpecifier)]
#[bits = 4]
pub enum TexCoordGenSource {
    Position,
    Normal,
    Color,
    Binormal,
    Tangent,
    Tex0,
    Tex1,
    Tex2,
    Tex3,
    Tex4,
    Tex5,
    Tex6,
    Tex7,
}

#[derive(BitfieldSpecifier)]
pub enum TexCoordGenMode {
    /// Transform the source row from the vertex by the texture matrix associated with this
    /// texgen slot.
    Regular,
    /// Compute an offset texture coordinate for embossing. This is the only mode that uses
    /// `emboss_source` and `emboss_light`.
    ///
    /// TODO: What precisely does this do?
    EmbossMap,
    /// Extracts (r, g, ?, ?) from color 0. Use with `source_row: Color`.
    ///
    /// TODO: Is the matrix multiply performed? Normalize and post matrix? What happens with
    /// other source rows?
    ColorSrTgC0,
    /// Extracts (r, g, ?, ?) from color 1. Use with `source_row: Color`.
    ///
    /// TODO: Is the matrix multiply performed? Normalize and post matrix? What happens with
    /// other source rows?
    ColorSrTgC1,
}

#[derive(BitfieldSpecifier)]
pub enum TexCoordGenInputForm {
    /// Extract (s, t, 1, 1) from the input.
    Ab11,
    /// Extract (x, y, z, 1) from the input. Only position, normal, binormal, and tangent
    /// provide a meaningful third component.
    Abc1,
}

#[derive(BitfieldSpecifier)]
pub enum TexCoordProjection {
    /// Use two rows for a 2x4 texture matrix.
    St,
    /// Use three rows for a 3x4 texture matrix. The result will be projected to two components
    /// for texturing.
    Stq,
}

pub struct XfTexCoordGenRegB {
    index: u8,
}

impl XfTexCoordGenRegB {
    pub const TEX0: Self = Self { index: 0 };
    pub const TEX1: Self = Self { index: 1 };
    pub const TEX2: Self = Self { index: 2 };
    pub const TEX3: Self = Self { index: 3 };
    pub const TEX4: Self = Self { index: 4 };
    pub const TEX5: Self = Self { index: 5 };
    pub const TEX6: Self = Self { index: 6 };
    pub const TEX7: Self = Self { index: 7 };

    pub fn new(index: u8) -> Option<Self> {
        if index <= 7 {
            Some(Self { index })
        } else {
            None
        }
    }
}

impl XfReg for XfTexCoordGenRegB {
    type T = TexCoordGenRegB;

    fn addr(&self) -> u16 {
        0x1050 | self.index as u16
    }
}

#[bitfield]
#[derive(Clone, Copy)]
pub struct TexCoordGenRegB {
    #[skip]
    unused: B23,
    pub normalize: bool,
    #[skip]
    unused: B2,
    pub post_transform_matrix: PostTransformMatrix,
}

impl_from!(TexCoordGenRegB, u32);
