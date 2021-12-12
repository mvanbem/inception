use std::collections::HashMap;

use anyhow::Result;
use texture_atlas::{PatchId, RgbU8Image, TextureAtlas};

use crate::bsp::Bsp;

#[derive(Clone, Copy, Debug)]
pub struct LightmapMetadata {
    pub luxel_offset: [usize; 2],
    pub is_flipped: bool,
}

pub struct LightmapPatch {
    pub width: u16,
    pub height: u16,
    pub style_count: u8,
    pub bump_light: bool,
    pub data_by_style_axis: Vec<Vec<u8>>,
}

pub struct ClusterLightmap {
    pub width: usize,
    pub height: usize,
    pub metadata_by_data_offset: HashMap<i32, LightmapMetadata>,
    pub image: RgbU8Image,
}

#[derive(Default)]
struct ClusterLightmapBuilder {
    atlas: TextureAtlas,
    patch_ids_by_data_offset: HashMap<i32, PatchId>,
}

impl ClusterLightmapBuilder {
    fn build(self, bsp: Bsp, cluster: i16) -> ClusterLightmap {
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

        // TODO: Remove this in the packer and have the client fill in its own light maps dynamically.
        let mut image_data = vec![0; 3 * width * height];
        let atlas_width = width;
        let atlas_height = height;
        for leaf in bsp.iter_worldspawn_leaves() {
            if leaf.cluster != cluster {
                continue;
            }

            for face in bsp.iter_faces_from_leaf(leaf) {
                if face.light_ofs == -1 || face.tex_info == -1 {
                    continue;
                }
                let tex_info = &bsp.tex_infos()[face.tex_info as usize];
                let style_count: u8 = face
                    .styles
                    .iter()
                    .map(|&x| if x != 255 { 1 } else { 0 })
                    .sum();
                assert!(style_count > 0);
                let bump_light = (tex_info.flags & 0x800) != 0;

                let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;
                let metadata = metadata_by_data_offset[&face.light_ofs];

                // Convert the luxel data.
                // There can be up to four styles. Higher indexed styles come first.
                let mut src_offset = face.light_ofs as i32
                    + (4 * width * height * (style_count as usize - 1)) as i32;
                for y in 0..height {
                    for x in 0..width {
                        let (dst_x, dst_y) = if metadata.is_flipped { (y, x) } else { (x, y) };
                        let dst_offset = 3
                            * (atlas_width * (dst_y + metadata.luxel_offset[1])
                                + dst_x
                                + metadata.luxel_offset[0]);
                        image_data[dst_offset..dst_offset + 3]
                            .copy_from_slice(&bsp.lighting().at_offset(src_offset, 4)[0].to_rgb8());
                        src_offset += 4;
                    }
                }
            }
        }

        ClusterLightmap {
            width,
            height,
            metadata_by_data_offset,
            image: RgbU8Image::new(atlas_width, atlas_height, image_data),
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
        .map(|(cluster, builder)| (cluster, builder.build(bsp, cluster)))
        .collect();

    Ok(cluster_lightmaps)
}
