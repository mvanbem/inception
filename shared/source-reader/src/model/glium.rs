use std::rc::Rc;

use glium::index::PrimitiveType;
use glium::{implement_vertex, Display, IndexBuffer};

use crate::asset::vmt::Vmt;
use crate::asset::AssetLoader;
use crate::model::mdl::Mdl;
use crate::model::vtx::Vtx;
use crate::model::vvd::Vvd;
use crate::vpk::path::VpkPath;

#[cfg(feature = "glium")]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

implement_vertex!(Vertex, position, normal, tex_coord);

pub struct Batch {
    pub index_buffer: IndexBuffer<u16>,
    pub base_map: Option<Rc<Vmt>>,
}

pub fn build_vertex_buffer(
    display: &Display,
    asset_loader: &AssetLoader,
    mdl: Mdl,
    vtx: Vtx,
    vvd: Vvd,
) -> (Vec<Vertex>, Vec<Batch>) {
    const LOD: i32 = 0;

    let mut materials = Vec::new();
    for (i, texture) in mdl.textures().iter().enumerate() {
        println!("Texture {}: {}", i, texture.name(mdl));
        materials.push(
            asset_loader
                .get_material(&VpkPath::new_with_prefix_and_extension(
                    texture.name(mdl),
                    // TODO: Build this from the model path.
                    "materials/models/police",
                    "vmt",
                ))
                .ok(),
        );
    }

    let mut vertex_data = Vec::new();
    for fixup in vvd.fixups() {
        if fixup.lod >= LOD {
            for i in 0..fixup.num_vertexes as usize {
                let src_index = fixup.source_vertex_id as usize + i;
                let vertex = vvd.vertex(src_index);
                vertex_data.push(Vertex {
                    position: vertex.position,
                    normal: vertex.normal,
                    tex_coord: vertex.tex_coord,
                });
            }
        }
    }

    let mdl_body_parts = mdl.body_parts();

    let mut batches = Vec::new();
    for (body_part_index, vtx_body_part) in vtx.body_parts().iter().enumerate() {
        // println!("VTX body part: {vtx_body_part:?}");
        let mdl_body_part = &mdl_body_parts[body_part_index];
        // println!("MDL body part: {mdl_body_part:?}");

        for (model_index, vtx_model) in vtx_body_part.models(vtx).iter().enumerate() {
            // println!("VTX model: {vtx_model:?}");
            let lod = &vtx_model.lods(vtx)[LOD as usize];
            // println!("VTX lod: {lod:?}");
            let mdl_model = &mdl_body_part.models(mdl)[model_index];
            // println!("MDL model: {mdl_model:?}");

            for (mesh_index, vtx_mesh) in lod.iter_meshes(vtx).enumerate() {
                // println!("VTX mesh: {vtx_mesh:?}");
                let mdl_mesh = &mdl_model.meshes(mdl)[mesh_index];
                // println!("MDL mesh: {mdl_mesh:?}");

                let base_map = materials[mdl_mesh.material as usize].as_ref();

                for strip_group in vtx_mesh.iter_strip_groups() {
                    // println!("VTX strip group: {strip_group:?}");

                    for strip in strip_group.iter_strips() {
                        // println!("VTX strip: {strip:?}");

                        let mut index_data = Vec::new();
                        for i in 0..strip.num_indices() as usize {
                            let strip_index = strip.index_offset() as usize + i;
                            let strip_group_index = strip_group.index(strip_index);
                            let orig_mesh_vert_id = strip_group
                                .vert(strip_group_index as usize)
                                .orig_mesh_vert_id();
                            let index = mdl_model.vertexindex / 48
                                + mdl_mesh.vertexoffset
                                + orig_mesh_vert_id as i32;

                            index_data.push(index as u16);
                        }

                        let primitive_type = if strip.flags().is_trilist() {
                            PrimitiveType::TrianglesList
                        } else if strip.flags().is_tristrip() {
                            PrimitiveType::TriangleStrip
                        } else {
                            unreachable!();
                        };
                        let index_buffer =
                            IndexBuffer::new(display, primitive_type, &index_data).unwrap();
                        batches.push(Batch {
                            index_buffer,
                            base_map: base_map.map(Rc::clone),
                        });
                    }
                }
            }
        }
    }

    (vertex_data, batches)
}
