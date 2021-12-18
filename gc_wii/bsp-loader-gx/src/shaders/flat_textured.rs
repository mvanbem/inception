use crate::gx::*;
use crate::shader::*;

pub static FLAT_TEXTURED_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP0),
        )
        .build(),
    ind_tex_stages: [None; 4],
    num_chans: 0,
    tex_gens: [
        // Texture coord.
        Some(TexGen::new(
            TexGenType::Mtx2x4,
            TexGenSrc::Tex0,
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
