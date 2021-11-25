use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::iter::repeat_with;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use byteorder::{BigEndian, WriteBytesExt};
use clap::{clap_app, crate_authors, crate_description, crate_version};
use memmap::Mmap;
use source_reader::asset::AssetLoader;
use source_reader::bsp::Bsp;
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::{FallbackFileLoader, FileLoader};
use source_reader::geometry::convert_vertex;
use source_reader::lightmap::{build_lightmaps, Lightmap};
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;
use try_insert_ext::EntryInsertExt;

use crate::counter::U16Counter;
use crate::display_list::DisplayListBuilder;

mod counter;
mod display_list;

fn main() -> Result<()> {
    let matches = clap_app!(app =>
        (name: "inception-pack")
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg hl2_base: --("hl2-base") <PATH> "Path to a Half-Life 2 installation")
        (@arg dst: --dst [PATH] "Path to write packed outputs (default: .)")
    )
    .get_matches();

    let hl2_base: &Path = Path::new(matches.value_of("hl2_base").unwrap());
    let map_path = hl2_base.join("maps/d1_trainstation_01.bsp");
    let bsp_file =
        File::open(&map_path).with_context(|| format!("Opening map file {:?}", map_path))?;
    let bsp_data = unsafe { Mmap::map(&bsp_file) }?;
    let bsp = Bsp::new(&bsp_data);
    let asset_loader = build_asset_loader(hl2_base, bsp)?;

    let lightmap = build_lightmaps(bsp)?;
    let ProcessedGeometry {
        position_data,
        lightmap_coord_data: texcoord_data,
        cluster_display_lists,
    } = process_geometry(bsp, &lightmap, asset_loader)?;

    let dst_path = Path::new(matches.value_of("dst").unwrap_or("."));
    create_dir_all(dst_path)?;

    write_lightmap(dst_path, lightmap)?;
    write_position_data(dst_path, position_data)?;
    write_lightmap_coord_data(dst_path, texcoord_data)?;
    write_display_lists(dst_path, cluster_display_lists)?;
    write_bsp_nodes(dst_path, bsp)?;
    write_bsp_leaves(dst_path, bsp)?;
    write_vis(dst_path, bsp)?;

    Ok(())
}

struct ProcessedGeometry {
    position_data: Vec<u8>,
    lightmap_coord_data: Vec<u8>,
    cluster_display_lists: Vec<DisplayListBuilder>,
}

fn process_geometry(
    bsp: Bsp,
    lightmap: &Lightmap,
    asset_loader: AssetLoader,
) -> Result<ProcessedGeometry> {
    let mut position_indices = HashMap::new();
    let mut lightmap_coord_indices = HashMap::new();
    let mut position_counter = U16Counter::new();
    let mut texcoord_counter = U16Counter::new();
    let mut position_data = Vec::new();
    let mut lightmap_coord_data = Vec::new();
    let mut cluster_display_lists = repeat_with(|| DisplayListBuilder::new())
        .take(bsp.leaves().len())
        .collect::<Vec<_>>();
    for leaf in bsp.iter_worldspawn_leaves() {
        if leaf.cluster == -1 {
            // Leaf is not potentially visible from anywhere.
            continue;
        }
        let mut batch_builder =
            cluster_display_lists[leaf.cluster as usize].build_batch(DisplayListBuilder::TRIANGLES);

        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs == -1 || face.tex_info == -1 {
                // Not a textured lightmapped surface.
                continue;
            }

            let lightmap_metadata = &lightmap.metadata_by_data_offset[&face.light_ofs];
            let tex_info = &bsp.tex_infos()[face.tex_info as usize];
            if tex_info.tex_data == -1 {
                // Not textured.
                // TODO: Determine whether any such faces need to be drawn.
                continue;
            }

            // This is a textured face.
            let tex_data = &bsp.tex_datas()[tex_info.tex_data as usize];
            let material_path = VpkPath::new_with_prefix_and_extension(
                bsp.tex_data_strings()
                    .get(tex_data.name_string_table_id as usize),
                "materials",
                "vmt",
            );
            let _material = asset_loader.get_material(&material_path)?;

            let mut first_position_index = None;
            let mut first_texcoord_index = None;
            let mut prev_position_index = None;
            let mut prev_texcoord_index = None;
            for vertex_index in bsp.iter_vertex_indices_from_face(face) {
                let vertex = convert_vertex(
                    bsp,
                    &lightmap.image,
                    lightmap_metadata,
                    face,
                    tex_info,
                    vertex_index,
                );

                let position_index = *position_indices
                    .entry(hashable_float(&vertex.position))
                    .or_try_insert_with(|| -> Result<_> {
                        write_floats(&mut position_data, &vertex.position)?;
                        Ok(position_counter.next())
                    })?;
                let lightmap_coord_index = *lightmap_coord_indices
                    .entry(hashable_float(&vertex.lightmap_coord))
                    .or_try_insert_with(|| -> Result<_> {
                        write_floats(&mut lightmap_coord_data, &vertex.lightmap_coord)?;
                        Ok(texcoord_counter.next())
                    })?;

                if first_position_index.is_none() {
                    first_position_index = Some(position_index);
                    first_texcoord_index = Some(lightmap_coord_index);
                }

                if prev_position_index.is_some() {
                    let mut data = [0; 12];
                    let mut w = &mut data[..];
                    w.write_u16::<BigEndian>(first_position_index.unwrap())?;
                    w.write_u16::<BigEndian>(first_texcoord_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_position_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_texcoord_index.unwrap())?;
                    w.write_u16::<BigEndian>(position_index)?;
                    w.write_u16::<BigEndian>(lightmap_coord_index)?;
                    batch_builder.emit_vertices(3, &data);
                }
                prev_position_index = Some(position_index);
                prev_texcoord_index = Some(lightmap_coord_index);
            }
        }
    }
    Ok(ProcessedGeometry {
        position_data,
        lightmap_coord_data,
        cluster_display_lists,
    })
}

fn build_asset_loader<'a>(hl2_base: &Path, bsp: Bsp<'a>) -> Result<AssetLoader<'a>> {
    let pak_loader = Rc::new(ZipArchiveLoader::new(bsp.pak_file()));
    let material_loader = Rc::new(FallbackFileLoader::new(vec![
        Rc::clone(&pak_loader) as Rc<dyn FileLoader>,
        Rc::new(Vpk::new(hl2_base.join("hl2_misc"))?),
    ]));
    let texture_loader = Rc::new(FallbackFileLoader::new(vec![
        Rc::clone(&pak_loader) as Rc<dyn FileLoader>,
        Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?),
    ]));
    Ok(AssetLoader::new(material_loader, texture_loader))
}

#[derive(Clone, Copy)]
struct FloatByBits(f32);

impl PartialEq for FloatByBits {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FloatByBits {}

impl Hash for FloatByBits {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

fn hashable_float<const N: usize>(array: &[f32; N]) -> [FloatByBits; N] {
    let mut result = [FloatByBits(0.0); N];
    for index in 0..N {
        result[index] = FloatByBits(array[index]);
    }
    result
}

fn write_floats<const N: usize>(data: &mut Vec<u8>, array: &[f32; N]) -> Result<()> {
    for value in array {
        data.write_f32::<BigEndian>(*value)?;
    }
    Ok(())
}

fn write_lightmap(dst_path: &Path, lightmap: Lightmap) -> Result<()> {
    lightmap
        .image
        .write_to_png(dst_path.join("lightmap.png").to_str().unwrap())?;
    Ok(())
}

fn write_position_data(dst_path: &Path, position_data: Vec<u8>) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("position_data.dat"))?);
    f.write_all(&position_data)?;
    f.flush()?;
    Ok(())
}

fn write_lightmap_coord_data(dst_path: &Path, texcoord_data: Vec<u8>) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("texcoord_data.dat"))?);
    f.write_all(&texcoord_data)?;
    f.flush()?;
    Ok(())
}

fn write_display_lists(
    dst_path: &Path,
    cluster_display_lists: Vec<DisplayListBuilder>,
) -> Result<()> {
    let mut built_display_lists = Vec::new();
    let mut offset = (8 * cluster_display_lists.len() as u32 + 31) & !31;
    let mut index = Vec::new();
    for display_list in cluster_display_lists {
        let built_display_list = display_list.build();
        let len = built_display_list.len() as u32;
        index
            .write_u32::<BigEndian>(if len > 0 { offset } else { 0 })
            .unwrap();
        index.write_u32::<BigEndian>(len).unwrap();
        offset += len;
        built_display_lists.push(built_display_list);
    }
    while (index.len() & 31) != 0 {
        index.push(0);
    }
    let mut f = BufWriter::new(File::create(dst_path.join("display_lists.dat"))?);
    f.write_all(&index)?;
    for display_list in built_display_lists {
        f.write_all(&display_list)?;
    }
    f.flush()?;
    Ok(())
}

fn write_bsp_nodes(dst_path: &Path, bsp: Bsp) -> Result<()> {
    let mut data = Vec::new();
    for node in bsp.nodes() {
        let plane = &bsp.planes()[node.planenum as usize];
        data.write_f32::<BigEndian>(plane.normal[0]).unwrap();
        data.write_f32::<BigEndian>(plane.normal[1]).unwrap();
        data.write_f32::<BigEndian>(plane.normal[2]).unwrap();
        data.write_f32::<BigEndian>(plane.dist).unwrap();
        data.write_i32::<BigEndian>(node.children[0]).unwrap();
        data.write_i32::<BigEndian>(node.children[1]).unwrap();
    }
    let mut f = BufWriter::new(File::create(dst_path.join("bsp_nodes.dat"))?);
    f.write_all(&data)?;
    f.flush()?;
    Ok(())
}

fn write_bsp_leaves(dst_path: &Path, bsp: Bsp) -> Result<()> {
    let mut data = Vec::new();
    for leaf in bsp.leaves() {
        data.write_i16::<BigEndian>(leaf.cluster).unwrap();
    }
    let mut f = BufWriter::new(File::create(dst_path.join("bsp_leaves.dat"))?);
    f.write_all(&data)?;
    f.flush()?;
    Ok(())
}

fn write_vis(dst_path: &Path, bsp: Bsp) -> Result<()> {
    let mut sized_vis_chunks = Vec::new();
    for cluster in bsp.visibility().iter_clusters() {
        sized_vis_chunks.push(cluster.find_data());
    }
    let mut offset = 4 * sized_vis_chunks.len() as u32 + 4;
    let mut index = Vec::new();
    index
        .write_u32::<BigEndian>(sized_vis_chunks.len() as u32)
        .unwrap();
    for &chunk in &sized_vis_chunks {
        index.write_u32::<BigEndian>(offset).unwrap();
        offset += chunk.len() as u32;
    }
    let mut f = BufWriter::new(File::create(dst_path.join("vis.dat"))?);
    f.write_all(&index)?;
    for &chunk in &sized_vis_chunks {
        f.write_all(chunk)?;
    }
    f.flush()?;
    Ok(())
}
