use crate::gx::*;
use crate::shader::*;

/// LightmappedGeneric, base alpha packed as aux alpha, env mapped, env map mask packed as auxiliary
/// intensity.
pub static LIGHTMAPPED_BAAA_ENV_EMAI_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Sample the env map.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex(TevTexCoord::TexCoord2, TevTexMap::TEXMAP2),
        )
        // Sample the aux map for alpha and to mask the env map.
        .add_stage(
            TevStage::new(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor)
                    .with_dst(TevReg::Reg0),
                TevStageAlpha::just(TevAlphaIn::TexAlpha),
            )
            .with_tex(TevTexCoord::TexCoord1, TevTexMap::TEXMAP3),
        )
        // Square the env map value for `$envmapcontrast 1`.
        .add_stage(TevStage::color_only(
            TevStageColor::mul(TevColorIn::Reg0Color, TevColorIn::Reg0Color).with_dst(TevReg::Reg0),
        ))
        // Sample the lightmap.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex(TevTexCoord::TexCoord0, TevTexMap::TEXMAP0),
        )
        // Sample the base map and multiply it by the lightmap.
        .add_stage(
            TevStage::color_only(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor)
                    // Scale to allow the lightmap to over-brighten to some degree.
                    .with_scale(TevScale::K2),
            )
            .with_tex(TevTexCoord::TexCoord1, TevTexMap::TEXMAP1),
        )
        // Add the env map scaled by the env map tint.
        .add_stage(TevStage::color_only(
            TevStageColor::add_mul(
                TevColorIn::PrevColor,
                TevColorIn::Reg0Color,
                TevColorIn::Konst,
            )
            // Env map tint is expected in K0.
            .with_konst_sel(Some(TevColorKonst::K0Rgb)),
        ))
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
        // Environment map coordinates.
        //
        // TEXMTX0: View pos translation.
        // DTTMTX0: World space normalized vector to texture coordinate matrix.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX0,
            )
            .with_normalize(true)
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX0),
        ),
        None,
        None,
        None,
        None,
        None,
    ],
    swap_table: [[0, 1, 2, 3]; 4],
};
