use crate::gx::*;
use crate::shader::*;

pub static LIGHTMAPPED_ENV_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Sample the env map.
        .add_stage(
            TevStage::color_only(
                TevStageColor::mul(TevColorIn::TexColor, TevColorIn::Konst)
                    .with_dst(TevReg::Reg0)
                    // Env map tint is expected in K0.
                    .with_konst_sel(Some(TevColorKonst::K0Rgb)),
            )
            .with_tex_coord(TevTexCoord::TexCoord2)
            .with_tex_map(TevTexMap::TEXMAP2),
        )
        // Sample the lightmap.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP0),
        )
        // Sample the base map, multiply it by the lightmap, and add the env map.
        .add_stage(
            TevStage::new(
                TevStageColor::add_mul(
                    TevColorIn::Reg0Color,
                    TevColorIn::PrevColor,
                    TevColorIn::TexColor,
                )
                // Arbitrary scale to get things in range.
                .with_scale(TevScale::K2),
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
