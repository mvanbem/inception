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
                let tex_info = &bsp.tex_infos()[face.tex_info as usize];
                let style_count: i32 = face
                    .styles
                    .iter()
                    .map(|&x| if x != 255 { 1 } else { 0 })
                    .sum();
                assert!(style_count > 0);
                let _is_bump_mapped = (tex_info.flags & 0x800) != 0;

                // Import the lightmap patch if it hasn't already been imported.
                if !patch_ids_by_data_offset.contains_key(&face.light_ofs) {
                    // Allocate a patch in the lightmap texture atlas.
                    let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                    let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;

                    // Convert the luxel data.
                    // There can be up to four styles. Higher indexed styles come first. Skip ahead
                    // to get style 0.
                    // TODO: Do something with bump mapped data.
                    let offset =
                        face.light_ofs as i32 + 4 * (width * height) as i32 * (style_count - 1);
                    let data = bsp
                        .lighting()
                        .at_offset(offset, width * height)
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
