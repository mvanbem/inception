use crate::gx::*;
use crate::shader::*;

pub static LIGHTMAPPED_BUMP_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Compute `n dot l_0` clamped to [0, 1] by indirect texturing.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor).with_dst(TevReg::Reg0))
                .with_tex_coord(TevTexCoord::TexCoord1)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(1 /* select red */),
        )
        // Multiply the above term by the first lightmap.
        .add_stage(
            TevStage::color_only(
                TevStageColor::mul(TevColorIn::Reg0Color, TevColorIn::TexColor).with_clamp(false),
            )
            .with_tex_coord(TevTexCoord::TexCoord0)
            .with_tex_map(TevTexMap::TEXMAP0),
        )
        // Compute `n dot l_1` clamped to [0, 1] by indirect texturing.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor).with_dst(TevReg::Reg0))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(2 /* select green */),
        )
        // Multiply the above term by the second lightmap.
        .add_stage(
            TevStage::color_only(
                TevStageColor::add_mul(
                    TevColorIn::PrevColor,
                    TevColorIn::Reg0Color,
                    TevColorIn::TexColor,
                )
                .with_clamp(false),
            )
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP4),
        )
        // Compute `n dot l_1` clamped to [0, 1] by indirect texturing.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor).with_dst(TevReg::Reg0))
                .with_tex_coord(TevTexCoord::TexCoord1)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(1 /* select red */),
        )
        // Multiply the above term by the third lightmap.
        .add_stage(
            TevStage::color_only(
                TevStageColor::add_mul(
                    TevColorIn::PrevColor,
                    TevColorIn::Reg0Color,
                    TevColorIn::TexColor,
                )
                .with_clamp(true)
                .with_scale(TevScale::K2 /* arbitrary boost */),
            )
            .with_tex_coord(TevTexCoord::TexCoord0)
            .with_tex_map(TevTexMap::TEXMAP5),
        )
        // Sample the base map and multiply it in.
        .add_stage(
            TevStage::new(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor),
                TevStageAlpha::just(TevAlphaIn::TexAlpha),
            )
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP1),
        )
        .build(),
    ind_tex_stages: [
        // Sample the bump map. This stage is configured elsewhere to produce `n dot l` values for
        // the first and second bump lightmap basis vectors.
        Some(IndTexStage::new(TevTexCoord::TexCoord1, TevTexMap::TEXMAP3)),
        // Sample the bump map. This stage is configured elsewhere to produce the `n dot l` value
        // for the third bump lightmap basis vector.
        Some(IndTexStage::new(TevTexCoord::TexCoord1, TevTexMap::TEXMAP3)),
        None,
        None,
    ],
    num_chans: 0,
    tex_gens: [
        // Lightmap coord.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex0,
            TexMtxIndex::IDENTITY,
        )),
        // Base and bump map coord.
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
    swap_table: [
        // Regular RGBA.
        [0, 1, 2, 3],
        // RRRA: first and third lightmap dot product
        [0, 0, 0, 3],
        // GGGA: second lightmap dot product
        [1, 1, 1, 3],
        // Unused
        [0, 1, 2, 3],
    ],
};
