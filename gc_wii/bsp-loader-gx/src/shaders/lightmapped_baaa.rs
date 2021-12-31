use crate::gx::*;
use crate::shader::*;

/// LightmappedGeneric, base alpha packed as aux alpha.
pub static LIGHTMAPPED_BAAA_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Sample the lightmap.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP0),
        )
        // Sample the base map and multiply it by the lightmap.
        .add_stage(
            TevStage::new(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor)
                    // Arbitrary scale to get things in range.
                    .with_scale(TevScale::K2),
                TevStageAlpha::just(TevAlphaIn::TexAlpha),
            )
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP1),
        )
        // Sample the aux map for alpha.
        .add_stage(
            TevStage::new(
                TevStageColor::just(TevColorIn::PrevColor),
                TevStageAlpha::just(TevAlphaIn::TexAlpha),
            )
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP3),
        )
        .build(),
    ind_tex_stages: [None, None, None, None],
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
