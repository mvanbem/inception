use core::mem::zeroed;

use ogc_sys::*;

use crate::gx::*;
use crate::shader::*;

// Names for the swap table entries defined below.
const _SWAP_RGBA: u8 = 0;
const _SWAP_RRRB: u8 = 1;
const _SWAP_GGGB: u8 = 2;
const _SWAP_BBRB: u8 = 3;

/// Computes the view space reflection vector. Highly expensive.
pub static _REFLECTION_VECTOR_SHADER: Shader = Shader {
    stages: tev_builder()
        // Naive dot product 1/6.
        // Load the view space normal X component from texgen.
        //
        // texc = vec3(u(normal.x));
        //
        // prevc <- texc;
        //        = vec3(u(normal.x));
        //
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(_SWAP_RRRB),
        )
        // Naive dot product 2/6.
        // Multiply in the view space view vector X component from texgen.
        //
        // prevc = vec3(u(normal.x));
        // texc  = vec3(u(view.x));
        //
        // r0c <- prevc * texc;
        //      = vec3(u(normal.x) * u(view.x));
        //
        .add_stage(
            TevStage::color_only(
                TevStageColor::mul(TevColorIn::PrevColor, TevColorIn::TexColor)
                    .with_dst(TevReg::Reg0),
            )
            .with_tex_coord(TevTexCoord::TexCoord2)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_RRRB),
        )
        // Naive dot product 3/6.
        // Load the view space normal Y component from texgen.
        //
        // r0c = vec3(u(normal.x) * u(view.x));
        //
        // texc = vec3(u(normal.y));
        //
        // prevc <- texc;
        //        = vec3(u(normal.y));
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(_SWAP_GGGB),
        )
        // Naive dot product 4/6.
        // Multiply in the view space view vector Y component from texgen.
        //
        // r0c   = vec3(u(normal.x) * u(view.x));
        // prevc = vec3(u(normal.y));
        // texc  = vec3(u(view.y));
        //
        // r0c <- r0c + prevc * texc;
        //      = vec3(u(normal.x) * u(view.x) + u(normal.y) * u(view.y));
        .add_stage(
            TevStage::color_only(
                TevStageColor::add_mul(
                    TevColorIn::Reg0Color,
                    TevColorIn::PrevColor,
                    TevColorIn::TexColor,
                )
                .with_clamp(false)
                .with_dst(TevReg::Reg0),
            )
            .with_tex_coord(TevTexCoord::TexCoord2)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_GGGB),
        )
        // Naive dot product 5/6.
        // Load the view space normal Z component from texgen.
        //
        // r0c = vec3(u(normal.x) * u(view.x) + u(normal.y) * u(view.y));
        //
        // texc = vec3(u(normal.z));
        //
        // prevc <- texc;
        //        = vec3(u(normal.z));
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord1)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(_SWAP_RRRB),
        )
        // Naive dot product 6/6.
        // Multiply in the view space view vector Z component from texgen.
        //
        // r0c   = vec3(u(normal.x) * u(view.x) + u(normal.y) * u(view.y));
        // prevc = vec3(u(normal.z));
        // texc  = vec3(u(view.z));
        //
        // r0c <- r0c + prevc * texc;
        //      = vec3(u(normal.x) * u(view.x) + u(normal.y) * u(view.y) + u(normal.z) * u(view.z));
        //      = vec3(dot(u(normal), u(view)));
        .add_stage(
            TevStage::color_only(
                TevStageColor::add_mul(
                    TevColorIn::Reg0Color,
                    TevColorIn::PrevColor,
                    TevColorIn::TexColor,
                )
                .with_clamp(false)
                .with_dst(TevReg::Reg0),
            )
            .with_tex_coord(TevTexCoord::TexCoord3)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_RRRB),
        )
        // The naive dot product contains unwanted terms that are linear in the normal and view
        // vectors. Subtract out the first correction factor from texgen.
        //
        // r0c  = vec3(naive_dot_product);
        // texc = vec3(normal_correction_factor);
        //
        // r0c <- r0c - texc;
        //      = vec3(naive_dot_product - normal_correction_factor);
        //
        .add_stage(
            TevStage::color_only(
                TevStageColor::sub(TevColorIn::Reg0Color, TevColorIn::TexColor)
                    .with_clamp(false)
                    .with_dst(TevReg::Reg0),
            )
            .with_tex_coord(TevTexCoord::TexCoord4)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_RRRB),
        )
        // Subtract out the second correction factor from texgen.
        //
        // r0c  = vec3(naive_dot_product - normal_correction_factor);
        // texc = vec3(view_correction_factor);
        //
        // r0c <- r0c - texc;
        //      = vec3(naive_dot_product - normal_correction_factor - view_correction_factor);
        //      = vec3(dot(normal, view));
        //
        .add_stage(
            TevStage::color_only(
                TevStageColor::sub(TevColorIn::Reg0Color, TevColorIn::TexColor)
                    .with_bias(TevBias::AddHalf)
                    .with_scale(TevScale::K2)
                    .with_clamp(true)
                    .with_dst(TevReg::Reg0),
            )
            .with_tex_coord(TevTexCoord::TexCoord5)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_RRRB),
        )
        // Move the corrected dot product into [-1, 1].
        .add_stage(TevStage::color_only(
            TevStageColor::just(TevColorIn::Reg0Color)
                .with_bias(TevBias::SubHalf)
                .with_scale(TevScale::K2)
                .with_clamp(false)
                .with_dst(TevReg::Reg0),
        ))
        // Load the view space normal XY components from texgen, to be combined with the Z component
        // in the next stage.
        //
        // r0c = vec3(dot(normal, view));
        //
        // texc = vec3(normal.xy, 0);
        //
        // prevc <- texc;
        //        = vec3(normal.xy, 0);
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord0)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(_SWAP_RGBA),
        )
        // Load the view space normal Z component from texgen to complete it.
        //
        // r0c   = vec3(dot(normal, view));
        // prevc = vec3(normal.xy, 0);
        // texc = vec3(0, 0, normal.z);
        //
        // prevc <- vec3(normal.xy, normal.z);
        //        = normal;
        //
        .add_stage(
            TevStage::color_only(TevStageColor::add(
                TevColorIn::PrevColor,
                TevColorIn::TexColor,
            ))
            .with_tex_coord(TevTexCoord::TexCoord1)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_BBRB),
        )
        // Multiply the dot product by the normal and double it.
        //
        // r0c   = vec3(dot(normal, view));
        // prevc = normal;
        //
        // r0c <- 2 * r0c * prevc;
        //      = 2 * dot(normal, view) * normal;
        //
        // Note that r0c may be out of byte range! This is fine as long as it's sent to a 10-bit
        // input.
        //
        .add_stage(TevStage::color_only(
            TevStageColor::mul(TevColorIn::Reg0Color, TevColorIn::PrevColor)
                .with_scale(TevScale::K2)
                .with_clamp(false)
                .with_dst(TevReg::Reg0),
        ))
        // Load the view space view vector XY components from texgen.
        //
        // r0c  = 2 * dot(normal, view) * normal;  // S10
        // texc = vec3(view.xy, 0);
        //
        // prevc <- texc;
        //
        .add_stage(
            TevStage::color_only(TevStageColor::just(TevColorIn::TexColor))
                .with_tex_coord(TevTexCoord::TexCoord2)
                .with_tex_map(TevTexMap::TEXMAP7)
                .with_tex_swap(_SWAP_RGBA),
        )
        // Load the view space view vector Z component from texgen to complete it.
        //
        // r0c   = 2 * dot(normal, view) * normal;  // S10
        // prevc = vec3(view.xy, 0);
        // texc  = vec3(0, 0, view.z);
        //
        // prevc <- prevc + texc;
        //        = vec3(view.xy, view.z);
        //        = view;
        //
        .add_stage(
            TevStage::color_only(TevStageColor::add(
                TevColorIn::PrevColor,
                TevColorIn::TexColor,
            ))
            .with_tex_coord(TevTexCoord::TexCoord3)
            .with_tex_map(TevTexMap::TEXMAP7)
            .with_tex_swap(_SWAP_BBRB),
        )
        // Finish the reflection vector calculation.
        //
        // r0c   = 2 * dot(normal, view) * normal;  // S10
        // prevc = view;
        //
        // prevc <- r0c - prevc;
        //        = 2 * dot(normal, view) * normal - view;
        //        = reflected;
        //
        .add_stage(TevStage::color_only(
            TevStageColor::sub(TevColorIn::Reg0Color, TevColorIn::PrevColor)
                .with_clamp(false)
                .with_scale(TevScale::K1_2),
        ))
        // Scale and bias the vector into color range.
        //
        // prevc = reflected;
        //
        // prevc <- (prevc + 1) * 0.5;
        //        = (reflected + 1) * 0.5;
        //
        .add_stage(TevStage::color_only(
            TevStageColor::add(TevColorIn::PrevColor, TevColorIn::Constant1)
                .with_scale(TevScale::K1_2),
        ))
        .build(),
    num_chans: 0,
    tex_gens: [
        // For passing the view space normal XY components through an identity texture.
        Some(
            TexGen::new(TexGenType::Mtx3x4, TexGenSrc::Normal, TexMtxIndex::TEXMTX0)
                .with_normalize(true)
                .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX0),
        ),
        // For passing the view space normal Z component through an identity texture.
        Some(
            TexGen::new(TexGenType::Mtx3x4, TexGenSrc::Normal, TexMtxIndex::TEXMTX0)
                .with_normalize(true)
                .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX1),
        ),
        // For constructing the view space view vector XY components with an identity texture.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX1,
            )
            .with_normalize(true)
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX0),
        ),
        // For constructing the view space view vector Z component with an identity texture.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX1,
            )
            .with_normalize(true)
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX1),
        ),
        // For constructing the normal dot product correction factor with an identity texture.
        Some(
            TexGen::new(TexGenType::Mtx3x4, TexGenSrc::Normal, TexMtxIndex::TEXMTX0)
                .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX2),
        ),
        // For constructing the view dot product correction factor with an identity texture.
        Some(
            TexGen::new(
                TexGenType::Mtx3x4,
                TexGenSrc::Position,
                TexMtxIndex::TEXMTX1,
            )
            .with_post_mtx_index(PostTransformTexMtxIndex::DTTMTX2),
        ),
        None,
        None,
    ],
    swap_table: [
        // RGBA as usual
        [0, 1, 2, 3],
        // RRRB
        [0, 0, 0, 2],
        // GGGB
        [1, 1, 1, 2],
        // BBRB
        [2, 2, 0, 2],
    ],
};

fn _set_reflection_vector_matrices(
    view: &mut Mtx,
    yaw_rotation: &mut Mtx,
    pitch_rotation: &mut Mtx,
) {
    unsafe {
        let mut tmp = zeroed::<Mtx>();
        let mut world_to_eye = [
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
        ];
        let mut scale_and_bias_xy1 = [
            [0.5, 0.0, 0.0, 0.5],
            [0.0, 0.5, 0.0, 0.5],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut scale_and_bias_z01 = [
            [0.0, 0.0, 0.5, 0.5],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut dot_product_correction_factor =
            [[0.25, 0.25, 0.25, 0.5], [0.0; 4], [0.0, 0.0, 0.0, 1.0]];
        let mut normal = zeroed::<Mtx>();
        c_guMtxConcat(
            yaw_rotation.as_mut_ptr(),
            world_to_eye.as_mut_ptr(),
            tmp.as_mut_ptr(),
        );
        c_guMtxConcat(
            pitch_rotation.as_mut_ptr(),
            tmp.as_mut_ptr(),
            normal.as_mut_ptr(),
        );
        GX_LoadTexMtxImm(normal.as_mut_ptr(), GX_TEXMTX0, GX_MTX3x4 as u8);
        GX_LoadTexMtxImm(view.as_mut_ptr(), GX_TEXMTX1, GX_MTX3x4 as u8);
        GX_LoadTexMtxImm(scale_and_bias_xy1.as_mut_ptr(), GX_DTTMTX0, GX_MTX3x4 as u8);
        GX_LoadTexMtxImm(scale_and_bias_z01.as_mut_ptr(), GX_DTTMTX1, GX_MTX3x4 as u8);
        GX_LoadTexMtxImm(
            dot_product_correction_factor.as_mut_ptr(),
            GX_DTTMTX2,
            GX_MTX3x4 as u8,
        );
    }
}
