use gamecube_shader::gx::*;
use gamecube_shader::*;

pub static WORLD_VERTEX_TRANSITION_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Sample the first base map.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex(TevTexCoord::TexCoord1, TevTexMap::TEXMAP1),
        )
        // Sample the second base map and blend between them by the rasterized alpha.
        .add_stage(
            TevStage::color_only(TevStageColor::mix(
                TevColorIn::PrevColor,
                TevColorIn::TexColor,
                TevColorIn::RasColor,
            ))
            .with_tex(TevTexCoord::TexCoord2, TevTexMap::TEXMAP2)
            .with_channel(TevChannel::Color0),
        )
        // Sample the lightmap and multiply it in.
        .add_stage(
            TevStage::color_only(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor)
                    // Scale to allow the lightmap to over-brighten to some degree.
                    .with_scale(TevScale::K2),
            )
            .with_tex(TevTexCoord::TexCoord0, TevTexMap::TEXMAP0),
        )
        .build(),
    ind_tex_stages: [None; 4],
    num_chans: 1,
    tex_gens: [
        // Lightmap coord.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex0,
            TexMtxIndex::IDENTITY,
        )),
        // Texture coord 1.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex1,
            TexMtxIndex::IDENTITY,
        )),
        // Texture coord 2.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex2,
            TexMtxIndex::IDENTITY,
        )),
        None,
        None,
        None,
        None,
        None,
    ],
    swap_table: [[0, 1, 2, 3]; 4],
};
