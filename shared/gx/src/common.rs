use modular_bitfield_msb::prelude::*;

macro_rules! impl_from {
    ($t:ty, $prim:ty) => {
        impl From<$t> for $prim {
            fn from(value: $t) -> Self {
                <$prim>::from_be_bytes(value.into_bytes())
            }
        }
    };
}

/// The first of two collections of matrix indices. The value for a CP register and an XF register.
#[bitfield]
#[derive(Clone, Copy)]
pub struct MatrixRegA {
    #[skip]
    unused: B2,
    pub tex3: TextureMatrix,
    pub tex2: TextureMatrix,
    pub tex1: TextureMatrix,
    pub tex0: TextureMatrix,
    pub geometry: GeometryMatrix,
}

impl_from!(MatrixRegA, u32);

/// The second of two collections of matrix indices. The value for a CP register and an XF register.
#[bitfield]
#[derive(Clone, Copy)]
pub struct MatrixRegB {
    #[skip]
    unused: B8,
    pub tex7: TextureMatrix,
    pub tex6: TextureMatrix,
    pub tex5: TextureMatrix,
    pub tex4: TextureMatrix,
}

impl_from!(MatrixRegB, u32);

#[derive(BitfieldSpecifier)]
#[bits = 6]
pub enum GeometryMatrix {
    PNMTX0 = 0,
    PNMTX1 = 3,
    PNMTX2 = 6,
    PNMTX3 = 9,
    PNMTX4 = 12,
    PNMTX5 = 15,
    PNMTX6 = 18,
    PNMTX7 = 21,
    PNMTX8 = 24,
    PNMTX9 = 27,
}

#[derive(BitfieldSpecifier)]
#[bits = 6]
pub enum TextureMatrix {
    IDENTITY = 60,
    TEXMTX0 = 30,
    TEXMTX1 = 33,
    TEXMTX2 = 36,
    TEXMTX3 = 39,
    TEXMTX4 = 42,
    TEXMTX5 = 45,
    TEXMTX6 = 48,
    TEXMTX7 = 51,
    TEXMTX8 = 54,
    TEXMTX9 = 57,
}

#[derive(BitfieldSpecifier)]
#[bits = 6]
pub enum PostTransformMatrix {
    IDENTITY = 61,
    DTTMTX0 = 0,
    DTTMTX1 = 3,
    DTTMTX2 = 6,
    DTTMTX3 = 9,
    DTTMTX4 = 12,
    DTTMTX5 = 15,
    DTTMTX6 = 18,
    DTTMTX7 = 21,
    DTTMTX8 = 24,
    DTTMTX9 = 27,
}
