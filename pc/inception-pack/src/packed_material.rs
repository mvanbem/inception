use std::collections::BTreeMap;

use anyhow::{bail, Result};
use source_reader::asset::vmt::{LightmappedGeneric, Shader, Vmt};
use source_reader::asset::AssetLoader;
use texture_format::TextureFormat;

use crate::texture_key::{BorrowedTextureKey, TextureIdAllocator};
use crate::Plane;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackedMaterial {
    pub base_id: u16,
    pub aux_id: Option<u16>,
    pub base_alpha: PackedMaterialBaseAlpha,
    pub env_map: Option<PackedMaterialEnvMap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PackedMaterialBaseAlpha {
    BaseTextureAlpha,
    AuxTextureAlpha,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackedMaterialEnvMap {
    pub ids_by_plane: BTreeMap<Plane, u16>,
    pub mask: PackedMaterialEnvMapMask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PackedMaterialEnvMapMask {
    None,
    BaseTextureAlpha,
    AuxTextureIntensity,
}

impl From<PackedMaterialBaseAlpha> for PackedMaterialEnvMapMask {
    fn from(base_alpha: PackedMaterialBaseAlpha) -> Self {
        match base_alpha {
            PackedMaterialBaseAlpha::BaseTextureAlpha => PackedMaterialEnvMapMask::BaseTextureAlpha,
            // NOTE: This would make more obvious sense as "AuxTextureAlpha", but when base alpha is
            // packed as an I8 texture, that value can be read on any channel.
            PackedMaterialBaseAlpha::AuxTextureAlpha => {
                PackedMaterialEnvMapMask::AuxTextureIntensity
            }
        }
    }
}

impl PackedMaterial {
    pub fn from_material_and_all_planes<'a, I>(
        asset_loader: &AssetLoader,
        ids: &mut TextureIdAllocator,
        material: &Vmt,
        planes: I,
    ) -> Result<Option<Self>>
    where
        I: IntoIterator,
        I::IntoIter: Iterator<Item = &'a Plane>,
    {
        Ok(match material.shader() {
            // Generic path
            Shader::LightmappedGeneric(LightmappedGeneric {
                base_alpha_env_map_mask,
                base_texture_path,
                bump_map_path,
                env_map_mask_path,
                env_map_path,
                normal_map_alpha_env_map_mask,
                ..
            }) => {
                let base_texture = asset_loader.get_texture(base_texture_path)?;
                let base_id = ids.get(&BorrowedTextureKey::EncodeAsIs {
                    texture_path: base_texture_path,
                });
                let base_alpha = match base_texture.format() {
                    TextureFormat::Dxt5 => PackedMaterialBaseAlpha::AuxTextureAlpha,
                    TextureFormat::Dxt1 => PackedMaterialBaseAlpha::BaseTextureAlpha,
                    format => bail!("unexpected base texture format: {:?}", format),
                };

                #[derive(Clone, Copy, PartialEq, Eq)]
                enum SourceChannel {
                    Intensity,
                    Alpha,
                }

                let (env_map, env_map_mask_for_aux_intensity) =
                    if let Some(env_map_path) = env_map_path {
                        let ids_by_plane: BTreeMap<Plane, u16> = planes
                            .into_iter()
                            .map(|plane| {
                                let id = ids.get(&BorrowedTextureKey::BakeOrientedEnvmap {
                                    texture_path: env_map_path,
                                    plane,
                                });
                                (*plane, id)
                            })
                            .collect();

                        match (
                            env_map_mask_path,
                            base_alpha_env_map_mask,
                            normal_map_alpha_env_map_mask,
                        ) {
                            // No env map mask.
                            (None, false, false) => (
                                Some(PackedMaterialEnvMap {
                                    ids_by_plane,
                                    mask: PackedMaterialEnvMapMask::None,
                                }),
                                None,
                            ),
                            // Dedicated env map mask.
                            (Some(env_map_mask_path), false, false) => (
                                Some(PackedMaterialEnvMap {
                                    ids_by_plane,
                                    mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                                }),
                                Some((env_map_mask_path, SourceChannel::Intensity)),
                            ),
                            // Base alpha env map mask.
                            (None, true, false) => (
                                Some(PackedMaterialEnvMap {
                                    ids_by_plane,
                                    mask: base_alpha.into(),
                                }),
                                // TODO: Need to keep a note that this is inverted!
                                None,
                            ),
                            // Normal map alpha env map mask.
                            (None, false, true) => (
                                Some(PackedMaterialEnvMap {
                                    ids_by_plane,
                                    mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                                }),
                                Some((
                                    // TODO: Make this an error rather than a panic.
                                    bump_map_path.as_ref().unwrap(),
                                    SourceChannel::Alpha,
                                )),
                            ),
                            _ => bail!(
                                "material {} has unexpected env map mask parameters: \
                                env_map_mask={} \
                                base_alpha_env_map_mask={} \
                                normal_map_alpha_env_map_mask={}",
                                material.path(),
                                env_map_mask_path.is_some(),
                                base_alpha_env_map_mask,
                                normal_map_alpha_env_map_mask,
                            ),
                        }
                    } else {
                        (None, None)
                    };

                // Compose the auxiliary texture, depending on which channels are in demand.
                let aux_id = match (base_alpha, env_map_mask_for_aux_intensity) {
                    // Zero channels. No aux map.
                    (PackedMaterialBaseAlpha::BaseTextureAlpha, None) => None,

                    // One channel. The requested data becomes an intensity texture, which can be
                    // read as a grey color or as alpha.
                    (PackedMaterialBaseAlpha::AuxTextureAlpha, None) => {
                        Some(ids.get(&BorrowedTextureKey::AlphaToIntensity {
                            texture_path: base_texture_path,
                        }))
                    }
                    (
                        PackedMaterialBaseAlpha::BaseTextureAlpha,
                        Some((env_map_mask_path, SourceChannel::Intensity)),
                    ) => Some(ids.get(&BorrowedTextureKey::Intensity {
                        texture_path: env_map_mask_path,
                    })),
                    (
                        PackedMaterialBaseAlpha::BaseTextureAlpha,
                        Some((env_map_mask_path, SourceChannel::Alpha)),
                    ) => Some(ids.get(&BorrowedTextureKey::AlphaToIntensity {
                        texture_path: env_map_mask_path,
                    })),

                    // Two channels. Build an intensity-alpha texture with the env map mask in the
                    // intensity channel and the base texture alpha in the alpha channel.
                    (
                        PackedMaterialBaseAlpha::AuxTextureAlpha,
                        Some((env_map_mask_path, env_map_mask_src_channel)),
                    ) => Some(ids.get(&BorrowedTextureKey::ComposeIntensityAlpha {
                        intensity_texture_path: env_map_mask_path,
                        intensity_from_alpha: env_map_mask_src_channel == SourceChannel::Alpha,
                        alpha_texture_path: base_texture_path,
                    })),
                };

                Some(Self {
                    base_id,
                    aux_id,
                    base_alpha,
                    env_map,
                })
            }

            Shader::Unsupported { .. } => None,
        })
    }
}
