#![allow(dead_code)]

use ogc_sys::*;
use paste::paste;
use seq_macro::seq;

use crate::gx::*;

macro_rules! tev_builder {
    (all) => {
        tev_builder!(0 => add(1));
        tev_builder!(1 => add(2) build());
        tev_builder!(2 => add(3) build());
        tev_builder!(3 => add(4) build());
        tev_builder!(4 => add(5) build());
        tev_builder!(5 => add(6) build());
        tev_builder!(6 => add(7) build());
        tev_builder!(7 => add(8) build());
        tev_builder!(8 => add(9) build());
        tev_builder!(9 => add(10) build());
        tev_builder!(10 => add(11) build());
        tev_builder!(11 => add(12) build());
        tev_builder!(12 => add(13) build());
        tev_builder!(13 => add(14) build());
        tev_builder!(14 => add(15) build());
        tev_builder!(15 => add(16) build());
        tev_builder!(16 => build());
    };
    ($n:expr => $(add($next_n:expr))? $(build $build:tt)?) => {
        paste! {
            pub struct [<TevBuilder $n>] {
                stages: [TevStage; $n],
            }

            impl [<TevBuilder $n>] {
                $(tev_builder!(impl_add ($n) ($next_n));)?
                $(tev_builder!(impl_build ($n) $build);)?
            }
        }
    };
    (impl_add ($n:expr) ($next_n:expr)) => {
        paste! {
            pub const fn add_stage(self, stage: TevStage) -> [<TevBuilder $next_n>] {
                [<TevBuilder $next_n>] {
                    stages: seq!(I in 0..$n {
                        [
                            #(self.stages[I],)*
                            stage,
                        ]
                    }),
                }
            }
        }
    };
    (impl_build ($n:expr) ()) => {
        pub const fn build(self) -> [Option<TevStage>; 16] {
            let mut stages = [None; 16];
            seq!(I in 0..$n {
                stages[I] = Some(self.stages[I]);
            });
            stages
        }
    };
}

tev_builder!(all);

pub const fn tev_builder() -> TevBuilder0 {
    TevBuilder0 { stages: [] }
}

#[derive(Clone, Debug)]
pub struct Shader {
    pub stages: [Option<TevStage>; 16],
    pub num_chans: u8,
    pub tex_gens: [Option<TexGen>; 8],
    pub swap_table: [[u8; 4]; 4],
}

impl Shader {
    pub const fn default() -> Self {
        Self {
            stages: [None; 16],
            num_chans: 0,
            tex_gens: [None; 8],
            swap_table: [[0, 1, 2, 3]; 4],
        }
    }

    pub fn apply(&self) {
        unsafe {
            let num_tev_stages = self.num_tev_stages();
            GX_SetNumTevStages(num_tev_stages);
            for (index, stage) in self.stages.iter().enumerate() {
                assert_eq!(stage.is_some(), (index as u8) < num_tev_stages);
                if let Some(stage) = stage.as_ref() {
                    stage.apply(index as u8);
                }
            }

            GX_SetNumChans(self.num_chans);

            let num_tex_gens = self
                .tex_gens
                .iter()
                .map(|tex_gen| if tex_gen.is_some() { 1 } else { 0 })
                .sum();
            GX_SetNumTexGens(num_tex_gens);
            for (index, tex_gen) in self.tex_gens.iter().enumerate() {
                assert_eq!(tex_gen.is_some(), (index as u32) < num_tex_gens);
                if let Some(tex_gen) = tex_gen.as_ref() {
                    tex_gen.apply(index as u16);
                }
            }

            for (index, swap) in self.swap_table.iter().enumerate() {
                GX_SetTevSwapModeTable(index as u8, swap[0], swap[1], swap[2], swap[3]);
            }
        }
    }

    pub fn num_tev_stages(&self) -> u8 {
        self.stages
            .iter()
            .map(|stage| if stage.is_some() { 1 } else { 0 })
            .sum()
    }

    #[allow(dead_code)]
    pub const fn make_slow(mut self) -> Self {
        self.stages[0] = Self::make_stage_slow(self.stages[0]);
        self.stages[1] = Self::make_stage_slow(self.stages[1]);
        self.stages[2] = Self::make_stage_slow(self.stages[2]);
        self.stages[3] = Self::make_stage_slow(self.stages[3]);
        self.stages[4] = Self::make_stage_slow(self.stages[4]);
        self.stages[5] = Self::make_stage_slow(self.stages[5]);
        self.stages[6] = Self::make_stage_slow(self.stages[6]);
        self.stages[7] = Self::make_stage_slow(self.stages[7]);
        self.stages[8] = Self::make_stage_slow(self.stages[8]);
        self.stages[9] = Self::make_stage_slow(self.stages[9]);
        self.stages[10] = Self::make_stage_slow(self.stages[10]);
        self.stages[11] = Self::make_stage_slow(self.stages[11]);
        self.stages[12] = Self::make_stage_slow(self.stages[12]);
        self.stages[13] = Self::make_stage_slow(self.stages[13]);
        self.stages[14] = Self::make_stage_slow(self.stages[14]);
        self.stages[15] = Self::make_stage_slow(self.stages[15]);
        self
    }

    #[allow(dead_code)]
    const fn make_stage_slow(stage: Option<TevStage>) -> Option<TevStage> {
        if stage.is_none() {
            Some(TevStage::pass())
        } else {
            stage
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TevStage {
    pub color: TevStageColor,
    pub alpha: TevStageAlpha,
    pub tex_coord: TevTexCoord,
    pub tex_map: TevTexMap,
    pub channel: TevChannel,
    pub ras_swap: u8,
    pub tex_swap: u8,
}

impl TevStage {
    pub const fn pass() -> Self {
        Self {
            color: TevStageColor::pass(),
            alpha: TevStageAlpha::pass(),
            tex_coord: TevTexCoord::Null,
            tex_map: TevTexMap::NULL,
            channel: TevChannel::Null,
            ras_swap: 0,
            tex_swap: 0,
        }
    }

    pub const fn new(color: TevStageColor, alpha: TevStageAlpha) -> Self {
        Self {
            color,
            alpha,
            ..Self::pass()
        }
    }

    pub const fn color_only(color: TevStageColor) -> Self {
        Self {
            color,
            ..Self::pass()
        }
    }

    pub const fn alpha_only(alpha: TevStageAlpha) -> Self {
        Self {
            alpha,
            ..Self::pass()
        }
    }

    pub const fn with_color_dst(self, dst: TevReg) -> Self {
        Self {
            color: TevStageColor { dst, ..self.color },
            ..self
        }
    }

    pub const fn with_tex_coord(self, tex_coord: TevTexCoord) -> Self {
        Self { tex_coord, ..self }
    }

    pub const fn with_tex_map(self, tex_map: TevTexMap) -> Self {
        Self { tex_map, ..self }
    }

    pub const fn with_channel(self, channel: TevChannel) -> Self {
        Self { channel, ..self }
    }

    pub const fn with_ras_swap(self, ras_swap: u8) -> Self {
        Self { ras_swap, ..self }
    }

    pub const fn with_tex_swap(self, tex_swap: u8) -> Self {
        Self { tex_swap, ..self }
    }

    pub fn apply(&self, stage: u8) {
        unsafe {
            GX_SetTevColorIn(
                stage,
                self.color.inputs[0] as u8,
                self.color.inputs[1] as u8,
                self.color.inputs[2] as u8,
                self.color.inputs[3] as u8,
            );
            GX_SetTevColorOp(
                stage,
                self.color.op as u8,
                self.color.bias as u8,
                self.color.scale as u8,
                self.color.clamp as u8,
                self.color.dst as u8,
            );
            if let Some(konst_sel) = self.color.konst_sel {
                GX_SetTevKColorSel(stage, konst_sel as u8);
            }
            GX_SetTevAlphaIn(
                stage,
                self.alpha.inputs[0] as u8,
                self.alpha.inputs[1] as u8,
                self.alpha.inputs[2] as u8,
                self.alpha.inputs[3] as u8,
            );
            GX_SetTevAlphaOp(
                stage,
                self.alpha.op as u8,
                self.alpha.bias as u8,
                self.alpha.scale as u8,
                self.alpha.clamp as u8,
                self.alpha.dst as u8,
            );
            if let Some(konst_sel) = self.alpha.konst_sel {
                GX_SetTevKAlphaSel(stage, konst_sel as u8);
            }
            GX_SetTevOrder(
                stage,
                self.tex_coord as u8,
                self.tex_map.as_u32(),
                self.channel as u8,
            );
            GX_SetTevSwapMode(stage, self.ras_swap, self.tex_swap);
        }
    }
}

pub trait ComponentIn: Copy {
    const ZERO: Self;
    const PREV: Self;
}

impl ComponentIn for TevColorIn {
    const ZERO: Self = Self::Constant0;
    const PREV: Self = Self::PrevColor;
}

impl ComponentIn for TevAlphaIn {
    const ZERO: Self = Self::Constant0;
    const PREV: Self = Self::PrevAlpha;
}

pub trait ComponentKonst: Copy {}

impl ComponentKonst for TevColorKonst {}

impl ComponentKonst for TevAlphaKonst {}

pub type TevStageColor = TevStageComponent<TevColorIn, TevColorKonst>;
pub type TevStageAlpha = TevStageComponent<TevAlphaIn, TevAlphaKonst>;

#[derive(Clone, Copy, Debug)]
pub struct TevStageComponent<Input: ComponentIn, Konst: ComponentKonst> {
    pub inputs: [Input; 4],
    pub op: TevOp,
    pub bias: TevBias,
    pub scale: TevScale,
    pub clamp: bool,
    pub dst: TevReg,
    pub konst_sel: Option<Konst>,
}

impl<Input: ComponentIn, Konst: ComponentKonst> TevStageComponent<Input, Konst> {
    pub const fn zero() -> Self {
        Self {
            inputs: [Input::ZERO; 4],
            op: TevOp::Add,
            bias: TevBias::Zero,
            scale: TevScale::K1,
            clamp: true,
            dst: TevReg::Prev,
            konst_sel: None,
        }
    }

    pub const fn pass() -> Self {
        Self::just(Input::PREV)
    }

    pub const fn just(input_10bit: Input) -> Self {
        Self {
            inputs: [Input::ZERO, Input::ZERO, Input::ZERO, input_10bit],
            ..Self::zero()
        }
    }

    pub const fn add(a_10bit: Input, b: Input) -> Self {
        Self {
            inputs: [b, Input::ZERO, Input::ZERO, a_10bit],
            ..Self::zero()
        }
    }

    pub const fn sub(a_10bit: Input, b: Input) -> Self {
        Self {
            inputs: [b, Input::ZERO, Input::ZERO, a_10bit],
            op: TevOp::Sub,
            ..Self::zero()
        }
    }

    pub const fn mul(a: Input, b: Input) -> Self {
        Self {
            inputs: [Input::ZERO, a, b, Input::ZERO],
            ..Self::zero()
        }
    }

    /// Computes a + b * c.
    pub const fn add_mul(a_10bit: Input, b: Input, c: Input) -> Self {
        Self {
            inputs: [Input::ZERO, b, c, a_10bit],
            ..Self::zero()
        }
    }

    /// Computes a - b * c.
    pub const fn sub_mul(a_10bit: Input, b: Input, c: Input) -> Self {
        Self {
            inputs: [Input::ZERO, b, c, a_10bit],
            op: TevOp::Sub,
            ..Self::zero()
        }
    }

    /// Computes (1 - c) * a + c * b.
    pub const fn mix(a: Input, b: Input, c: Input) -> Self {
        Self {
            inputs: [a, b, c, Input::ZERO],
            ..Self::zero()
        }
    }

    /// Computes (a.r > b.r ? c : 0).
    pub const fn comp_r8_gt(a: Input, b: Input, c: Input) -> Self {
        Self::comp_r8_gt_add(a, b, c, Input::ZERO)
    }

    /// Computes (a.r > b.r ? c : 0) + d.
    pub const fn comp_r8_gt_add(a: Input, b: Input, c: Input, d_10bit: Input) -> Self {
        Self {
            inputs: [a, b, c, d_10bit],
            op: TevOp::CompR8Gt,
            ..Self::zero()
        }
    }

    pub const fn with_bias(self, bias: TevBias) -> Self {
        Self { bias, ..self }
    }

    pub const fn with_scale(self, scale: TevScale) -> Self {
        Self { scale, ..self }
    }

    pub const fn with_clamp(self, clamp: bool) -> Self {
        Self { clamp, ..self }
    }

    pub const fn with_dst(self, dst: TevReg) -> Self {
        Self { dst, ..self }
    }

    pub const fn with_konst_sel(self, konst_sel: Option<Konst>) -> Self {
        Self { konst_sel, ..self }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TexGen {
    pub type_: TexGenType,
    pub src: TexGenSrc,
    pub mtx_index: TexMtxIndex,
    pub normalize: bool,
    pub post_mtx_index: PostTransformTexMtxIndex,
    pub cyl_wrap_s: bool,
    pub cyl_wrap_t: bool,
}

impl TexGen {
    pub const fn new(type_: TexGenType, src: TexGenSrc, mtx_index: TexMtxIndex) -> Self {
        Self {
            type_,
            src,
            mtx_index,
            normalize: false,
            post_mtx_index: PostTransformTexMtxIndex::IDENTITY,
            cyl_wrap_s: false,
            cyl_wrap_t: false,
        }
    }

    pub const fn with_normalize(self, normalize: bool) -> Self {
        Self { normalize, ..self }
    }

    pub const fn with_post_mtx_index(self, post_mtx_index: PostTransformTexMtxIndex) -> Self {
        Self {
            post_mtx_index,
            ..self
        }
    }

    pub const fn with_cyl_wrap_s(self, cyl_wrap_s: bool) -> Self {
        Self { cyl_wrap_s, ..self }
    }

    pub const fn with_cyl_wrap_t(self, cyl_wrap_t: bool) -> Self {
        Self { cyl_wrap_t, ..self }
    }

    pub fn apply(&self, tex_coord: u16) {
        unsafe {
            GX_SetTexCoordGen2(
                tex_coord,
                self.type_ as u32,
                self.src as u32,
                self.mtx_index.as_u32(),
                if self.normalize { GX_TRUE } else { GX_FALSE },
                self.post_mtx_index.as_u32(),
            );
            GX_SetTexCoordCylWrap(
                tex_coord as u8,
                if self.cyl_wrap_s { GX_TRUE } else { GX_FALSE } as u8,
                if self.cyl_wrap_t { GX_TRUE } else { GX_FALSE } as u8,
            )
        }
    }
}
