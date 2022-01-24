use nalgebra_glm::vec3;
use source_reader::asset::vmt::{
    LightmappedGeneric, Shader, UnlitGeneric, Vmt, WorldVertexTransition,
};

use crate::packed_material::{PackedMaterial, PackedMaterialBaseAlpha, PackedMaterialEnvMapMask};
use crate::Plane;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pass {
    LightmappedGeneric {
        alpha: PassAlpha,
        base_alpha: PackedMaterialBaseAlpha,
        env_map: Option<PassEnvMap>,
    },
    UnlitGeneric,
    SelfIllum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PassAlpha {
    OpaqueOrAlphaTest,
    AlphaBlend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PassEnvMap {
    pub mask: PackedMaterialEnvMapMask,
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
                env_map: packed_material
                    .env_map
                    .as_ref()
                    .map(|env_map| PassEnvMap { mask: env_map.mask }),
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
                    env_map: packed_material
                        .env_map
                        .as_ref()
                        .map(|env_map| PassEnvMap { mask: env_map.mask }),
                }
            }

            shader => panic!("unexpected shader for Pass: {:?}", shader.name()),
        }
    }

    pub fn as_mode(self) -> u8 {
        match self {
            // Disallowed combinations.
            Pass::LightmappedGeneric {
                alpha: _,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::BaseTextureAlpha,
                    }),
            } => unreachable!(
                "sampling base alpha for an env map mask, \
                but base alpha is packed in the aux texture"
            ),

            // # Opaque pass
            // ## Base texture alpha
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                env_map: None,
            } => 0,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::None,
                    }),
            } => 1,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::OpaqueOrAlphaTest,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::BaseTextureAlpha,
            //         }),
            // } => 2,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                    }),
            } => 3,

            // # Opaque pass (cont.)
            // ## Aux texture alpha
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map: None,
            } => 4,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::OpaqueOrAlphaTest,
            //     base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::None,
            //         }),
            // } => 5,
            // (disallowed) => 6,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                    }),
            } => 7,

            // # Blended pass
            // ## Base texture alpha
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map: None,
            // } => 8,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::None,
            //         }),
            // } => 9,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::BaseTextureAlpha,
            //         }),
            // } => 10,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
            //         }),
            // } => 11,

            // # Blended pass
            // ## Aux texture alpha
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map: None,
            } => 12,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::None,
                    }),
            } => 13,
            // (disallowed) => 14,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                    }),
            } => 15,

            Pass::UnlitGeneric => 16,

            Pass::SelfIllum => 17,

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
    // Order matters here! `plane` is the first field to minimize plane changes in the display byte
    // code.
    pub plane: Plane,
    pub env_map_tint: [u8; 3],
    pub alpha: ShaderParamsAlpha,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShaderParamsAlpha {
    Opaque,
    AlphaTest { threshold: u8 },
    AlphaBlend,
}

impl ShaderParams {
    pub fn from_material_plane(material: &Vmt, plane: Plane) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                alpha_test_reference,
                env_map_tint,
                translucent,
                ..
            }) => {
                let env_map_tint = env_map_tint.unwrap_or(vec3(1.0, 1.0, 1.0));
                Self {
                    plane,
                    env_map_tint: [
                        ((env_map_tint[0] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                        ((env_map_tint[1] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                        ((env_map_tint[2] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                    ],
                    alpha: match (*alpha_test, *translucent) {
                        (false, false) => ShaderParamsAlpha::Opaque,
                        (false, true) => ShaderParamsAlpha::AlphaBlend,
                        (true, false) => ShaderParamsAlpha::AlphaTest {
                            threshold: ((alpha_test_reference * 255.0).clamp(0.0, 255.0) + 0.5)
                                as u8,
                        },
                        (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                    },
                }
            }
            Shader::UnlitGeneric(UnlitGeneric { .. }) => Self {
                plane,
                env_map_tint: [255; 3],
                alpha: ShaderParamsAlpha::Opaque,
            },
            Shader::WorldVertexTransition(WorldVertexTransition { .. }) => Self {
                plane,
                env_map_tint: [255; 3],
                alpha: ShaderParamsAlpha::Opaque,
            },
            shader => panic!("unexpected shader {:?}", shader.name()),
        }
    }
}
