use crate::gx::*;
use crate::shader::*;

pub static SELF_ILLUM_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        // Sample the base map.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex(TevTexCoord::TexCoord0, TevTexMap::TEXMAP1),
        )
        // Sample the aux map, which contains the self-illum mask, and multiply it in.
        .add_stage(
            TevStage::new(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor),
                TevStageAlpha::just(TevAlphaIn::Constant0),
            )
            .with_tex(TevTexCoord::TexCoord0, TevTexMap::TEXMAP2),
        )
        .build(),
    ind_tex_stages: [None; 4],
    num_chans: 0,
    tex_gens: [
        // Texture coord.
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
        None,
    ],
    swap_table: [[0, 1, 2, 3]; 4],
};
