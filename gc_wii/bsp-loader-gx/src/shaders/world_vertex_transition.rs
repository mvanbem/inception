use crate::gx::*;
use crate::shader::*;

pub static WORLD_VERTEX_TRANSITION_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex(TevTexCoord::TexCoord0, TevTexMap::TEXMAP1),
        )
        .add_stage(
            TevStage::color_only(TevStageColor::mix(
                TevColorIn::PrevColor,
                TevColorIn::TexColor,
                TevColorIn::RasColor,
            ))
            .with_tex(TevTexCoord::TexCoord1, TevTexMap::TEXMAP2)
            .with_channel(TevChannel::Color0),
        )
        .build(),
    ind_tex_stages: [None; 4],
    num_chans: 1,
    tex_gens: [
        // Texture coord 1.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex0,
            TexMtxIndex::IDENTITY,
        )),
        // Texture coord 2.
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
