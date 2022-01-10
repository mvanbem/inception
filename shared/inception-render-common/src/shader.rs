/// Uniquely identifies a GX TEV configuration.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Shader {
    LightmappedGeneric {
        base_alpha: ShaderBaseAlpha,
        env_map: Option<ShaderEnvMap>,
    },
    UnlitGeneric,
    WorldVertexTransition,
}

impl Shader {
    pub fn to_u8(self) -> u8 {
        match self {
            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::BaseTextureAlpha,
                env_map: None,
            } => 0,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(ShaderEnvMap {
                        mask: ShaderEnvMapMask::None,
                    }),
            } => 1,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(ShaderEnvMap {
                        mask: ShaderEnvMapMask::BaseTextureAlpha,
                    }),
            } => 2,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(ShaderEnvMap {
                        mask: ShaderEnvMapMask::AuxTextureIntensity,
                    }),
            } => 3,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::AuxTextureAlpha,
                env_map: None,
            } => 4,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(ShaderEnvMap {
                        mask: ShaderEnvMapMask::None,
                    }),
            } => 5,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(ShaderEnvMap {
                        mask: ShaderEnvMapMask::BaseTextureAlpha,
                    }),
            } => 6,

            Self::LightmappedGeneric {
                base_alpha: ShaderBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(ShaderEnvMap {
                        mask: ShaderEnvMapMask::AuxTextureIntensity,
                    }),
            } => 7,

            Self::UnlitGeneric => 8,

            Self::WorldVertexTransition => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShaderBaseAlpha {
    BaseTextureAlpha,
    AuxTextureAlpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShaderEnvMap {
    pub mask: ShaderEnvMapMask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShaderEnvMapMask {
    None,
    BaseTextureAlpha,
    AuxTextureIntensity,
}
