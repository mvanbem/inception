use anyhow::Result;
use source_reader::asset::vmt::{
    LightmappedGeneric, Shader, UnlitGeneric, Vmt, WorldVertexTransition,
};
use source_reader::asset::AssetLoader;
use texture_format::TextureFormat;

use crate::texture_key::{BorrowedTextureKey, TextureIdAllocator};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackedMaterial {
    pub base_id: u16,
    pub aux_id: Option<u16>,
    pub base_alpha: PackedMaterialBaseAlpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PackedMaterialBaseAlpha {
    BaseTextureAlpha,
    AuxTextureAlpha,
}

impl PackedMaterial {
    pub fn from_material(
        asset_loader: &AssetLoader,
        ids: &mut TextureIdAllocator,
        material: &Vmt,
        for_displacement: bool,
    ) -> Result<Option<Self>> {
        Ok(match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                base_texture_path,
                self_illum: false,
                ..
            }) => {
                let base_texture = asset_loader.get_texture(base_texture_path)?;
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });
                let base_alpha = match base_texture.format() {
                    TextureFormat::Dxt5 => PackedMaterialBaseAlpha::AuxTextureAlpha,
                    TextureFormat::Dxt1 => PackedMaterialBaseAlpha::BaseTextureAlpha,
                    format => panic!("unexpected base texture format: {:?}", format),
                };

                // Compose the auxiliary texture, depending on which channels are in demand.
                let aux_id = match base_alpha {
                    // Zero channels. No aux map.
                    PackedMaterialBaseAlpha::BaseTextureAlpha => None,

                    // One channel. The requested data becomes an intensity texture, which can be
                    // read as a grey color or as alpha.
                    PackedMaterialBaseAlpha::AuxTextureAlpha => {
                        Some(ids.get(&BorrowedTextureKey::AlphaToIntensity {
                            texture_path: base_texture_path,
                        }))
                    }
                };

                Some(Self {
                    base_id,
                    aux_id,
                    base_alpha,
                })
            }

            Shader::LightmappedGeneric(LightmappedGeneric {
                base_texture_path,
                self_illum: true,
                ..
            }) => {
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });
                let aux_id = Some(ids.get(&BorrowedTextureKey::AlphaToIntensity {
                    texture_path: base_texture_path,
                }));

                Some(Self {
                    base_id,
                    aux_id,
                    base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                })
            }

            Shader::UnlitGeneric(UnlitGeneric {
                base_texture_path,
                self_illum: false,
            }) => {
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });

                Some(Self {
                    base_id,
                    aux_id: None,
                    base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                })
            }

            Shader::UnlitGeneric(UnlitGeneric {
                base_texture_path,
                self_illum: true,
            }) => {
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });
                let aux_id = Some(ids.get(&BorrowedTextureKey::AlphaToIntensity {
                    texture_path: base_texture_path,
                }));

                Some(Self {
                    base_id,
                    aux_id,
                    base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                })
            }

            Shader::WorldVertexTransition(WorldVertexTransition {
                base_texture_path,
                base_texture2_path,
                ..
            }) if for_displacement => {
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });
                let aux_id = Some(ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture2_path,
                }));

                Some(Self {
                    base_id,
                    aux_id,
                    base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                })
            }

            Shader::WorldVertexTransition(WorldVertexTransition {
                base_texture_path, ..
            }) if !for_displacement => {
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });

                Some(Self {
                    base_id,
                    aux_id: None,
                    base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                })
            }

            shader => {
                eprintln!(
                    "WARNING: Skipping shader for PackedMaterial: {}",
                    shader.name(),
                );
                None
            }
        })
    }
}
