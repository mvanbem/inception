use crate::gx::*;
use crate::shader::*;

pub static LIGHTMAPPED_ENV_SHADER: Shader = Shader {
    stages: tev_builder()
        // Sample the paraboloid Z component and emit a side selector.
        .add_stage(
            TevStage::color_only(
                TevStageColor::comp_r8_gt(
                    TevColorIn::TexColor,
                    TevColorIn::Constant1_2,
                    TevColorIn::Constant1,
                )
                .with_dst(TevReg::Reg0),
            )
            .with_tex_coord(TevTexCoord::TexCoord4)
            .with_tex_map(TevTexMap::TEXMAP7), // Identity map
        )
        // Sample the front paraboloid.
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord2)
                .with_tex_map(TevTexMap::TEXMAP5), // Debug paraboloid env map
        )
        // Sample the back paraboloid and combine it with the front sample.
        .add_stage(
            TevStage::color_only(
                TevStageColor::mix(
                    TevColorIn::TexColor,
                    TevColorIn::PrevColor,
                    TevColorIn::Reg0Color,
                )
                .with_dst(TevReg::Reg0)
                .with_scale(TevScale::K1_2),
            )
            .with_tex_coord(TevTexCoord::TexCoord3)
            .with_tex_map(TevTexMap::TEXMAP6), // Debug paraboloid env map
        )
        // Sample the lightmap.
        .add_stage(
            TevStage::color_only(
                TevStageColor::just(TevColorIn::TexColor).with_scale(TevScale::K1_2),
            )
            .with_tex_coord(TevTexCoord::TexCoord0)
            .with_tex_map(TevTexMap::TEXMAP0),
        )
        // Sample the base map, multiply it by the lightmap, and add the env map.
        .add_stage(
            TevStage::color_only(TevStageColor::add_mul(
                TevColorIn::Reg0Color,
                TevColorIn::PrevColor,
                TevColorIn::TexColor,
            ))
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP1),
        )
        .build(),
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
        // Front paraboloid env map coordinates.
        //
        // TEXMTX0: View pos translation.
        // DTTMTX0: Face front paraboloid reflection matrix.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX0,
            )
            .with_normalize(true)
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX0),
        ),
        // Back paraboloid env map coordinates.
        //
        // TEXMTX0: View pos translation.
        // DTTMTX0: Face back paraboloid reflection matrix.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX0,
            )
            .with_normalize(true)
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX1),
        ),
        // Paraboloid Z env map coordinates.
        //
        // TEXMTX0: View pos translation.
        // DTTMTX0: Face back paraboloid reflection matrix.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX0,
            )
            .with_normalize(true)
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX2),
        ),
        None,
        None,
        None,
    ],
    swap_table: [[0, 1, 2, 3]; 4],
};
