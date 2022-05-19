use gamecube_shader::gx::*;
use gamecube_shader::*;

pub static FLAT_VERTEX_COLOR_SHADER: Shader = Shader {
    tev_stages: tev_builder()
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::RasColor))
                .with_channel(TevChannel::Color0),
        )
        .build(),
    ind_tex_stages: [None; 4],
    num_chans: 1,
    tex_gens: [None; 8],
    swap_table: [[0, 1, 2, 3]; 4],
};
