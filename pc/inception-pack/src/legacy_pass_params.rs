use source_reader::asset::vmt::{
    LightmappedGeneric, Shader, UnlitGeneric, Vmt, WorldVertexTransition,
};

use crate::packed_material::{PackedMaterial, PackedMaterialBaseAlpha};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pass {
    LightmappedGeneric {
        alpha: PassAlpha,
        base_alpha: PackedMaterialBaseAlpha,
    },
    UnlitGeneric,
    SelfIllum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PassAlpha {
    OpaqueOrAlphaTest,
    AlphaBlend,
}

impl Pass {
    pub fn from_material(material: &Vmt, packed_material: &PackedMaterial) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                self_illum: false,
                translucent,
                ..
            }) => Self::LightmappedGeneric {
                alpha: match (*alpha_test, *translucent) {
                    (_, false) => PassAlpha::OpaqueOrAlphaTest,
                    (false, true) => PassAlpha::AlphaBlend,
                    (true, true) => {
                        panic!("material is both alpha-tested and alpha-blended")
                    }
                },
                base_alpha: packed_material.base_alpha,
            },

            Shader::LightmappedGeneric(LightmappedGeneric {
                self_illum: true, ..
            }) => Self::SelfIllum,

            Shader::UnlitGeneric(UnlitGeneric {
                self_illum: false, ..
            }) => Self::UnlitGeneric,

            Shader::UnlitGeneric(UnlitGeneric {
                self_illum: true, ..
            }) => Self::SelfIllum,

            Shader::WorldVertexTransition(WorldVertexTransition { .. }) => {
                Self::LightmappedGeneric {
                    alpha: PassAlpha::OpaqueOrAlphaTest,
                    base_alpha: packed_material.base_alpha,
                }
            }

            shader => panic!("unexpected shader for Pass: {:?}", shader.name()),
        }
    }

    pub fn as_mode(self) -> u8 {
        match self {
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            } => 0,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
            } => 1,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map: None,
            // } => 2,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
            } => 3,

            Pass::UnlitGeneric => 4,

            Pass::SelfIllum => 5,

            _ => panic!("unexpected pass: {:?}", self),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DisplacementPass {
    LightmappedGeneric,
    WorldVertexTransition,
}

impl DisplacementPass {
    pub fn as_mode(self) -> u8 {
        match self {
            Self::LightmappedGeneric => 0,
            Self::WorldVertexTransition => 1,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShaderParams {
    pub alpha: ShaderParamsAlpha,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShaderParamsAlpha {
    Opaque,
    AlphaTest { threshold: u8 },
    AlphaBlend,
}

impl ShaderParams {
    pub fn from_material(material: &Vmt) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                alpha_test_reference,
                translucent,
                ..
            }) => Self {
                alpha: match (*alpha_test, *translucent) {
                    (false, false) => ShaderParamsAlpha::Opaque,
                    (false, true) => ShaderParamsAlpha::AlphaBlend,
                    (true, false) => ShaderParamsAlpha::AlphaTest {
                        threshold: ((alpha_test_reference * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                    },
                    (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                },
            },
            Shader::UnlitGeneric(UnlitGeneric { .. }) => Self {
                alpha: ShaderParamsAlpha::Opaque,
            },
            Shader::WorldVertexTransition(WorldVertexTransition { .. }) => Self {
                alpha: ShaderParamsAlpha::Opaque,
            },
            shader => panic!("unexpected shader {:?}", shader.name()),
        }
    }
}
