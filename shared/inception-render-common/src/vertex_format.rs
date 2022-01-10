#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum VertexFormat {
    /// GX_VTXFMT0
    /// - GX_VA_POS, GX_POS_XYZ, GX_F32
    /// - GX_VA_NRM, GX_NRM_XYZ, GX_S8
    /// - GX_VA_TEX0, GX_TEX_ST, GX_U16, frac=15
    /// - GX_VA_TEX1, GX_TEX_ST, GX_S16, frac=8
    Brush = 0,

    /// GX_VTXFMT1
    /// - GX_VA_POS,  GX_POS_XYZ, GX_F32
    /// - GX_VA_CLR0, GX_CLR_RGB, GX_RGB8
    /// - GX_VA_TEX0, GX_TEX_ST,  GX_F32
    Displacement = 1,
}
