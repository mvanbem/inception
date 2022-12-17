use std::fs::{create_dir_all, File};
use std::io::{Cursor, Write};
use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use gx::display_list::{DisplayList, GxPrimitive};
use source_reader::asset::AssetLoader;
use source_reader::file::FileLoader;
use source_reader::model::mdl::Mdl;
use source_reader::model::vtx::Vtx;
use source_reader::model::vvd::Vvd;
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;

use crate::draw_builder::DrawBuilder;
use crate::gx_helpers::DisplayListExt;
use crate::write_big_endian::WriteBigEndian;

pub fn pack_model(hl2_base: &Path, dst: Option<&str>, model_name: Option<&str>) -> Result<()> {
    let model_name = model_name.unwrap_or("police");
    let hl2_misc_loader = Rc::new(Vpk::new(hl2_base.join("hl2_misc"))?);
    let hl2_textures_loader = Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?);
    let asset_loader = AssetLoader::new(
        Rc::clone(&hl2_misc_loader) as Rc<dyn FileLoader>,
        hl2_textures_loader,
    );

    let mdl_path = VpkPath::new_with_prefix_and_extension(model_name, "models", "mdl");
    let mdl_data = match hl2_misc_loader.load_file(&mdl_path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", mdl_path),
    };
    let mdl = Mdl::new(&mdl_data);
    println!("MDL Header: {:?}", mdl.header());
    println!("Name: {}", mdl.header().name());
    for (index, bone) in mdl.bones().iter().enumerate() {
        println!("Bone {}: {:?}", index, bone);
    }

    let vtx_path = VpkPath::new_with_prefix_and_extension(model_name, "models", "dx80.vtx");
    let vtx_data = match hl2_misc_loader.load_file(&vtx_path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", vtx_path),
    };
    let vtx = Vtx::new(&vtx_data);

    let vvd_path = VpkPath::new_with_prefix_and_extension(model_name, "models", "vvd");
    let vvd_data = match hl2_misc_loader.load_file(&vvd_path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", vvd_path),
    };
    let vvd = Vvd::new(&vvd_data);

    let (display_lists, references) =
        pack_model_geometry(&asset_loader, model_name, mdl, vtx, vvd)?;

    let dst_path = Path::new(dst.unwrap_or(".")).join("models");
    create_dir_all(&dst_path)?;

    let dst_file_name = format!("{}.dat", model_name);
    let mut file = File::create(&dst_path.join(dst_file_name))?;
    // TODO: Write model data.
    file.flush()?;

    Ok(())
}

pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl WriteBigEndian for Vertex {
    const SIZE: usize = 32;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        self.position.write_big_endian_to(w)?;
        self.normal.write_big_endian_to(w)?;
        self.tex_coord.write_big_endian_to(w)?;
        Ok(())
    }
}

pub struct ModelReferencesEntry {
    pub offset: u32,
    pub texture_hash: u32,
}

fn pack_model_geometry(
    asset_loader: &AssetLoader,
    model_name: &str,
    mdl: Mdl,
    vtx: Vtx,
    vvd: Vvd,
) -> Result<(Vec<u8>, Vec<ModelReferencesEntry>)> {
    const LOD: i32 = 0;

    let mut materials = Vec::new();
    for texture in mdl.textures().iter() {
        materials.push(
            asset_loader
                .get_material(&VpkPath::new_with_prefix_and_extension(
                    texture.name(mdl),
                    &format!("materials/models/{model_name}"),
                    "vmt",
                ))
                .ok(),
        );
    }

    let mut vertices = Vec::new();
    for fixup in vvd.fixups() {
        if fixup.lod >= LOD {
            for i in 0..fixup.num_vertexes as usize {
                let src_index = fixup.source_vertex_id as usize + i;
                let vertex = vvd.vertex(src_index);
                vertices.push(Vertex {
                    position: vertex.position,
                    normal: vertex.normal,
                    tex_coord: vertex.tex_coord,
                });
            }
        }
    }

    let mdl_body_parts = mdl.body_parts();

    let mut display_list = DisplayList::new();
    let mut buf = [0u8; Vertex::SIZE];
    for (body_part_index, vtx_body_part) in vtx.body_parts().iter().enumerate() {
        let mdl_body_part = &mdl_body_parts[body_part_index];

        for (model_index, vtx_model) in vtx_body_part.models(vtx).iter().enumerate() {
            let lod = &vtx_model.lods(vtx)[LOD as usize];
            let mdl_model = &mdl_body_part.models(mdl)[model_index];

            for (mesh_index, vtx_mesh) in lod.iter_meshes(vtx).enumerate() {
                let mdl_mesh = &mdl_model.meshes(mdl)[mesh_index];

                let base_map = &**materials[mdl_mesh.material as usize]
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow!(
                            "loading material {} in model {model_name:?}",
                            mdl_mesh.material,
                        )
                    })
                    .unwrap();
                display_list.append_bind_texture(0, todo!(), todo!());
                display_list.append_texcoord_scale(0, todo!(), todo!());

                for strip_group in vtx_mesh.iter_strip_groups() {
                    for strip in strip_group.iter_strips() {
                        let mut draw_builder = DrawBuilder::new(
                            if strip.flags().is_trilist() {
                                GxPrimitive::Triangles
                            } else if strip.flags().is_tristrip() {
                                GxPrimitive::TriangleStrip
                            } else {
                                unreachable!();
                            },
                            0,
                        );

                        for i in 0..strip.num_indices() as usize {
                            let strip_index = strip.index_offset() as usize + i;
                            let strip_group_index = strip_group.index(strip_index);
                            let orig_mesh_vert_id = strip_group
                                .vert(strip_group_index as usize)
                                .orig_mesh_vert_id();
                            let index = mdl_model.vertexindex / 48
                                + mdl_mesh.vertexoffset
                                + orig_mesh_vert_id as i32;

                            vertices[index as usize]
                                .write_big_endian_to(&mut Cursor::new(&mut buf[..]))?;
                            draw_builder.emit_vertices(1, &buf);
                        }
                        display_list
                            .commands
                            .extend_from_slice(&draw_builder.build().commands);
                    }
                }
            }
        }
    }

    let mut display_lists = Vec::new();
    let mut references = Vec::new();
    display_list.write_to(&mut display_lists, |display_lists, reference| {
        references.push(ModelReferencesEntry {
            offset: display_lists.len().try_into().unwrap(),
            texture_hash: todo!(),
        });
    });

    Ok((display_lists, references))
}
