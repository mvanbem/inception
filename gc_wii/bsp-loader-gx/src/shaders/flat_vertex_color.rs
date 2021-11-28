use crate::gx::*;
use crate::shader::*;

pub static FLAT_VERTEX_COLOR_SHADER: Shader = Shader {
    stages: tev_builder()
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::RasColor))
                .with_channel(TevChannel::Color0),
        )
        .build(),
    num_chans: 1,
    tex_gens: [None; 8],
    swap_table: [[0, 1, 2, 3]; 4],
};
