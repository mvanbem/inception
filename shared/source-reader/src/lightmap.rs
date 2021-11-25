use std::collections::HashMap;

use anyhow::Result;
use texture_atlas::{RgbU8Image, RgbU8TextureAtlas};

use crate::bsp::Bsp;

pub struct Lightmap {
    pub image: RgbU8Image,
    pub metadata_by_data_offset: HashMap<i32, LightmapMetadata>,
}

pub struct LightmapMetadata {
    pub luxel_offset: [usize; 2],
    pub is_flipped: bool,
}

pub fn build_lightmaps(bsp: Bsp) -> Result<Lightmap> {
    // Collect lightmap patches and insert them into a texture atlas.
    let mut lightmap_atlas = RgbU8TextureAtlas::new();
    let mut patch_ids_by_data_offset = HashMap::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs != -1 && face.tex_info != -1 {
                // Import the lightmap patch if it hasn't already been imported.
                if !patch_ids_by_data_offset.contains_key(&face.light_ofs) {
                    // Allocate a patch in the lightmap texture atlas.
                    let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                    let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;

                    // Convert the luxel data.
                    // TODO: There can be multiple lightmap sets per face! Handle them!
                    let data = bsp
                        .lighting()
                        .at_offset(face.light_ofs, width * height)
                        .iter()
                        .map(|sample| sample.to_rgb8())
                        .flatten()
                        .collect();
                    patch_ids_by_data_offset.insert(
                        face.light_ofs,
                        lightmap_atlas.insert(RgbU8Image::new(width, height, data)),
                    );
                }
            }
        }
    }

    // Bake the texture atlas and prepare the final index.
    let (image, offsets_by_patch_id) = lightmap_atlas.bake_smallest();
    let metadata_by_data_offset = patch_ids_by_data_offset
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

    Ok(Lightmap {
        image,
        metadata_by_data_offset,
    })
}
