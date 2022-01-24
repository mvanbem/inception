use crate::bsp::{Bsp, Face, TexInfo};
use crate::lightmap::Lightmap;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub lightmap_coord: [f32; 2],
    pub texture_coord: [f32; 2],
}

pub fn convert_vertex(
    bsp: Bsp,
    lightmap: Option<&Lightmap>,
    face: &Face,
    tex_info: &TexInfo,
    vertex_index: usize,
) -> Vertex {
    let plane = &bsp.planes()[face.plane_num as usize];
    let vertex = &bsp.vertices()[vertex_index];

    let normal = [plane.normal[0], plane.normal[1], plane.normal[2]];

    let (lightmap_s, lightmap_t) = if let Some(lightmap) = lightmap {
        if let Some(lightmap_metadata) = lightmap.metadata_by_data_offset.get(&face.light_ofs) {
            let patch_s = tex_info.lightmap_vecs[0][0] * vertex.x
                + tex_info.lightmap_vecs[0][1] * vertex.y
                + tex_info.lightmap_vecs[0][2] * vertex.z
                + tex_info.lightmap_vecs[0][3]
                - face.lightmap_texture_mins_in_luxels[0] as f32;
            let patch_t = tex_info.lightmap_vecs[1][0] * vertex.x
                + tex_info.lightmap_vecs[1][1] * vertex.y
                + tex_info.lightmap_vecs[1][2] * vertex.z
                + tex_info.lightmap_vecs[1][3]
                - face.lightmap_texture_mins_in_luxels[1] as f32;
            let (patch_s, patch_t) = if lightmap_metadata.is_flipped {
                (patch_t, patch_s)
            } else {
                (patch_s, patch_t)
            };
            (
                (patch_s + lightmap_metadata.luxel_offset[0] as f32 + 0.5) / lightmap.width as f32,
                (patch_t + lightmap_metadata.luxel_offset[1] as f32 + 0.5) / lightmap.height as f32,
            )
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    let texture_s = tex_info.texture_vecs[0][0] * vertex.x
        + tex_info.texture_vecs[0][1] * vertex.y
        + tex_info.texture_vecs[0][2] * vertex.z
        + tex_info.texture_vecs[0][3];
    let texture_t = tex_info.texture_vecs[1][0] * vertex.x
        + tex_info.texture_vecs[1][1] * vertex.y
        + tex_info.texture_vecs[1][2] * vertex.z
        + tex_info.texture_vecs[1][3];

    let vertex = Vertex {
        position: [vertex.x, vertex.y, vertex.z],
        normal,
        lightmap_coord: [lightmap_s, lightmap_t],
        texture_coord: [texture_s, texture_t],
    };
    vertex
}
