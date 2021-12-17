use std::collections::HashMap;

use anyhow::Result;
use texture_atlas::{PatchId, TextureAtlas};

use crate::bsp::Bsp;

#[derive(Clone, Copy, Debug)]
pub struct LightmapMetadata {
    pub luxel_offset: [usize; 2],
    pub is_flipped: bool,
}

pub struct LightmapPatch {
    pub width: u8,
    pub height: u8,
    pub style_count: u8,
    pub bump_light: bool,
    pub luxel_offset: [usize; 2],
    pub is_flipped: bool,
}

pub struct ClusterLightmap {
    pub width: usize,
    pub height: usize,
    pub metadata_by_data_offset: HashMap<i32, LightmapMetadata>,
}

#[derive(Default)]
struct ClusterLightmapBuilder {
    atlas: TextureAtlas,
    patch_ids_by_data_offset: HashMap<i32, PatchId>,
}

impl ClusterLightmapBuilder {
    fn build(self) -> ClusterLightmap {
        let (width, height, offsets_by_patch_id) = self.atlas.bake_smallest();
        let metadata_by_data_offset: HashMap<i32, LightmapMetadata> = self
            .patch_ids_by_data_offset
            .into_iter()
            .map(|(data_offset, patch_id)| {
                (
                    data_offset,
                    LightmapMetadata {
                        luxel_offset: offsets_by_patch_id[&patch_id],
                        is_flipped: patch_id.is_flipped(),
                    },
                )
            })
            .collect();

        ClusterLightmap {
            width,
            height,
            metadata_by_data_offset,
        }
    }
}

pub fn build_lightmaps(bsp: Bsp) -> Result<HashMap<i16, ClusterLightmap>> {
    // Lay out an abstract texture atlas for all of the lightmap patches in the map.
    let mut cluster_lightmap_builders: HashMap<i16, ClusterLightmapBuilder> = HashMap::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        if leaf.cluster == -1 {
            continue;
        }
        let lightmap_builder = cluster_lightmap_builders.entry(leaf.cluster).or_default();

        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs == -1 || face.tex_info == -1 {
                continue;
            }

            // Add the lightmap patch to the texture atlas once per unique light data offset.
            if !lightmap_builder
                .patch_ids_by_data_offset
                .contains_key(&face.light_ofs)
            {
                // Allocate a patch in the lightmap texture atlas.
                let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;
                lightmap_builder
                    .patch_ids_by_data_offset
                    .insert(face.light_ofs, lightmap_builder.atlas.insert(width, height));
            }
        }
    }

    // Bake texture atlases.
    let cluster_lightmaps: HashMap<i16, ClusterLightmap> = cluster_lightmap_builders
        .into_iter()
        .map(|(cluster, builder)| (cluster, builder.build()))
        .collect();

    Ok(cluster_lightmaps)
}
