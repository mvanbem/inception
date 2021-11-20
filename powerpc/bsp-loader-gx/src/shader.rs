use ogc_sys::*;

use crate::gx::*;

#[derive(Clone, Debug)]
pub struct Shader {
    pub stages: [Option<TevStage>; 16],
    pub num_chans: u8,
    pub tex_gens: [Option<TexGen>; 8],
}

impl Shader {
    pub fn apply(&self) {
        unsafe {
            GX_SetNumTevStages(
                self.stages
                    .iter()
                    .map(|stage| if stage.is_some() { 1 } else { 0 })
                    .sum(),
            );
            for (index, stage) in self.stages.iter().enumerate() {
                if let Some(stage) = stage.as_ref() {
                    stage.apply(index as u8);
                }
            }

            GX_SetNumChans(self.num_chans);

            GX_SetNumTexGens(
                self.tex_gens
                    .iter()
                    .map(|tex_gen| if tex_gen.is_some() { 1 } else { 0 })
                    .sum(),
            );
            for (index, tex_gen) in self.tex_gens.iter().enumerate() {
                if let Some(tex_gen) = tex_gen.as_ref() {
                    tex_gen.apply(index as u16);
                }
            }
        }
    }

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

    const fn make_stage_slow(stage: Option<TevStage>) -> Option<TevStage> {
        if stage.is_none() {
            Some(TevStage {
                color_in: [
                    TevColorIn::Constant0,
                    TevColorIn::Constant0,
                    TevColorIn::Constant0,
                    TevColorIn::PrevColor,
                ],
                color_op: TevOp::Add,
                color_bias: TevBias::Zero,
                color_scale: TevScale::K1,
                color_clamp: false,
                color_dst: TevReg::Prev,
                alpha_in: [
                    TevAlphaIn::Constant0,
                    TevAlphaIn::Constant0,
                    TevAlphaIn::Constant0,
                    TevAlphaIn::PrevAlpha,
                ],
                alpha_op: TevOp::Add,
                alpha_bias: TevBias::Zero,
                alpha_scale: TevScale::K1,
                alpha_clamp: false,
                alpha_dst: TevReg::Prev,
                tex_coord: TevTexCoord::Null,
                tex_map: TevTexMap::NULL,
                channel: TevChannel::Null,
            })
        } else {
            stage
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TevStage {
    pub color_in: [TevColorIn; 4],
    pub color_op: TevOp,
    pub color_bias: TevBias,
    pub color_scale: TevScale,
    pub color_clamp: bool,
    pub color_dst: TevReg,
    pub alpha_in: [TevAlphaIn; 4],
    pub alpha_op: TevOp,
    pub alpha_bias: TevBias,
    pub alpha_scale: TevScale,
    pub alpha_clamp: bool,
    pub alpha_dst: TevReg,
    pub tex_coord: TevTexCoord,
    pub tex_map: TevTexMap,
    pub channel: TevChannel,
}

impl TevStage {
    pub fn apply(&self, stage: u8) {
        unsafe {
            GX_SetTevColorIn(
                stage,
                self.color_in[0] as u8,
                self.color_in[1] as u8,
                self.color_in[2] as u8,
                self.color_in[3] as u8,
            );
            GX_SetTevColorOp(
                stage,
                self.color_op as u8,
                self.color_bias as u8,
                self.color_scale as u8,
                self.color_clamp as u8,
                self.color_dst as u8,
            );
            GX_SetTevAlphaIn(
                stage,
                self.alpha_in[0] as u8,
                self.alpha_in[1] as u8,
                self.alpha_in[2] as u8,
                self.alpha_in[3] as u8,
            );
            GX_SetTevAlphaOp(
                stage,
                self.alpha_op as u8,
                self.alpha_bias as u8,
                self.alpha_scale as u8,
                self.alpha_clamp as u8,
                self.alpha_dst as u8,
            );
            GX_SetTevOrder(
                stage,
                self.tex_coord as u8,
                self.tex_map.as_u32(),
                self.channel as u8,
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TexGen {
    pub type_: TexGenType,
    pub src: TexGenSrc,
    pub mtx_index: TexMtxIndex,
}

impl TexGen {
    pub fn apply(&self, tex_coord: u16) {
        unsafe {
            GX_SetTexCoordGen(
                tex_coord,
                self.type_ as u32,
                self.src as u32,
                self.mtx_index.as_u32(),
            );
        }
    }
}
