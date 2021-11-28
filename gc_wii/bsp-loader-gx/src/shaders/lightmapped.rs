use crate::gx::*;
use crate::shader::*;

pub static LIGHTMAPPED_SHADER: Shader = Shader {
    stages: tev_builder()
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP0),
        )
        .add_stage(
            TevStage::color_only(TevStageColor::mul(
                TevColorIn::PrevColor,
                TevColorIn::TexColor,
            ))
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP1),
        )
        .build(),
    num_chans: 0,
    tex_gens: [
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex0,
            TexMtxIndex::IDENTITY,
        )),
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
