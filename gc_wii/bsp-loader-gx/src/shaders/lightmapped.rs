use crate::gx::*;
use crate::shader::*;

pub static LIGHTMAPPED_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Sample the lightmap.
        .add_stage(
            TevStage::color_only(
                TevStageColor::just(TevColorIn::TexColor)
                    // Arbitrary scale to get things in range.
                    .with_scale(TevScale::K2),
            )
            .with_tex_coord(TevTexCoord::TexCoord0)
            .with_tex_map(TevTexMap::TEXMAP0),
        )
        // Sample the base map and multiply it by the lightmap.
        .add_stage(
            TevStage::new(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor),
                TevStageAlpha::just(TevAlphaIn::TexAlpha),
            )
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP1),
        )
        .build(),
    ind_tex_stages: [None; 4],
    num_chans: 0,
    tex_gens: [
        // Lightmap coord.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex0,
            TexMtxIndex::IDENTITY,
        )),
        // Base map coord.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex1,
            TexMtxIndex::IDENTITY,
        )),
        None,
        None,
        None,
        None,
        None,
        None,
    ],
    swap_table: [[0, 1, 2, 3]; 4],
};
