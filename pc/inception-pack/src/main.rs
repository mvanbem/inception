use std::collections::{BTreeMap, HashMap};
use std::fs::{create_dir_all, File};
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Seek, Write};
use std::iter::repeat_with;
use std::path::Path;
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::{clap_app, crate_authors, crate_description, crate_version};
use memmap::Mmap;
use nalgebra_glm::{make_vec3, reflect, vec3, vec4, Mat3, Mat3x4, Mat4};
use source_reader::asset::vmt::Shader;
use source_reader::asset::vtf::{ImageData, ImageFormat};
use source_reader::asset::AssetLoader;
use source_reader::bsp::Bsp;
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::{FallbackFileLoader, FileLoader};
use source_reader::geometry::convert_vertex;
use source_reader::lightmap::{build_lightmaps, Lightmap};
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;
use texture_atlas::RgbU8Image;
use try_insert_ext::EntryInsertExt;

use crate::counter::U16Counter;
use crate::display_list::DisplayListBuilder;
use crate::record_writer::RecordWriter;

mod counter;
mod display_list;
mod record_writer;

fn main() -> Result<()> {
    let matches = clap_app!(app =>
        (name: "inception-pack")
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg hl2_base: --("hl2-base") <PATH> "Path to a Half-Life 2 installation")
        (@arg map: --map [NAME] "Map name (default: d1_trainstation_01")
        (@arg dst: --dst [PATH] "Path to write packed outputs (default: .)")
    )
    .get_matches();

    let hl2_base: &Path = Path::new(matches.value_of("hl2_base").unwrap());
    let map_path = {
        let mut path = hl2_base.join("maps");
        path.push(format!(
            "{}.bsp",
            matches.value_of("map").unwrap_or("d1_trainstation_01"),
        ));
        path
    };
    let bsp_file =
        File::open(&map_path).with_context(|| format!("Opening map file {:?}", map_path))?;
    let bsp_data = unsafe { Mmap::map(&bsp_file) }?;
    let bsp = Bsp::new(&bsp_data);
    let asset_loader = build_asset_loader(hl2_base, bsp)?;

    let lightmap = build_lightmaps(bsp)?;
    let ProcessedGeometry {
        position_data,
        normal_data,
        lightmap_coord_data,
        texture_coord_data,
        display_lists_by_cluster_texture_plane,
        texture_ids,
        texture_paths,
    } = process_geometry(bsp, &lightmap, &asset_loader)?;

    let dst_path = Path::new(matches.value_of("dst").unwrap_or("."));
    create_dir_all(dst_path)?;

    write_lightmap(dst_path, lightmap)?;
    write_debug_env_maps(dst_path)?;
    write_textures(dst_path, &asset_loader, &texture_paths)?;
    write_position_data(dst_path, position_data)?;
    write_normal_data(dst_path, normal_data)?;
    write_lightmap_coord_data(dst_path, lightmap_coord_data)?;
    write_texture_coord_data(dst_path, texture_coord_data)?;
    write_display_lists(
        bsp,
        dst_path,
        texture_ids,
        display_lists_by_cluster_texture_plane,
    )?;
    write_bsp_nodes(dst_path, bsp)?;
    write_bsp_leaves(dst_path, bsp)?;
    write_vis(dst_path, bsp)?;

    Ok(())
}

struct ProcessedGeometry {
    position_data: Vec<u8>,
    normal_data: Vec<u8>,
    lightmap_coord_data: Vec<u8>,
    texture_coord_data: Vec<u8>,
    display_lists_by_cluster_texture_plane: Vec<BTreeMap<VpkPath, BTreeMap<u16, Vec<u8>>>>,
    texture_ids: HashMap<VpkPath, u16>,
    texture_paths: Vec<VpkPath>,
}

fn process_geometry(
    bsp: Bsp,
    lightmap: &Lightmap,
    asset_loader: &AssetLoader,
) -> Result<ProcessedGeometry> {
    let mut position_indices = HashMap::new();
    let mut normal_indices = HashMap::new();
    let mut lightmap_coord_indices = HashMap::new();
    let mut texture_coord_indices = HashMap::new();
    let mut position_counter = U16Counter::new();
    let mut normal_counter = U16Counter::new();
    let mut lightmap_coord_counter = U16Counter::new();
    let mut texture_coord_counter = U16Counter::new();
    let mut position_data = Vec::new();
    let mut normal_data = Vec::new();
    let mut lightmap_coord_data = Vec::new();
    let mut texture_coord_data = Vec::new();
    let mut display_lists_by_cluster_texture_plane: Vec<
        BTreeMap<VpkPath, BTreeMap<u16, DisplayListBuilder>>,
    > = repeat_with(|| BTreeMap::new())
        .take(bsp.leaves().len())
        .collect::<Vec<_>>();
    let mut texture_paths = Vec::new();
    let mut texture_ids = HashMap::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        if leaf.cluster == -1 {
            // Leaf is not potentially visible from anywhere.
            continue;
        }

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
            let material = asset_loader.get_material(&material_path)?;
            let base_texture = match material.shader() {
                Shader::LightmappedGeneric { base_texture, .. } => base_texture,
                // Not an implemented shader.
                _ => continue,
            };
            if !matches!(
                base_texture.data(),
                Some(ImageData {
                    format: ImageFormat::Dxt1,
                    ..
                })
            ) {
                // Not an implemented format.
                continue;
            }
            texture_ids
                .entry(base_texture.path().clone())
                .or_insert_with(|| {
                    let index = texture_paths.len() as u16;
                    texture_paths.push(base_texture.path().clone());
                    index
                });
            let display_lists_by_plane = display_lists_by_cluster_texture_plane
                [leaf.cluster as usize]
                .entry(base_texture.path().clone())
                .or_default();
            let batch_builder = display_lists_by_plane
                .entry(face.plane_num)
                .or_insert_with(|| DisplayListBuilder::new(DisplayListBuilder::TRIANGLES));

            let mut first_position_index = None;
            let mut first_normal_index = None;
            let mut first_lightmap_coord_index = None;
            let mut first_texture_coord_index = None;
            let mut prev_position_index = None;
            let mut prev_normal_index = None;
            let mut prev_lightmap_coord_index = None;
            let mut prev_texture_coord_index = None;
            for vertex_index in bsp.iter_vertex_indices_from_face(face) {
                let mut vertex = convert_vertex(
                    bsp,
                    &lightmap.image,
                    lightmap_metadata,
                    face,
                    tex_info,
                    vertex_index,
                );
                vertex.texture_coord = [
                    vertex.texture_coord[0] / base_texture.width() as f32,
                    vertex.texture_coord[1] / base_texture.height() as f32,
                ];

                let position_index = *position_indices
                    .entry(hashable_float(&vertex.position))
                    .or_try_insert_with(|| -> Result<_> {
                        write_floats(&mut position_data, &vertex.position)?;
                        Ok(position_counter.next())
                    })?;
                let normal_index = *normal_indices
                    .entry(hashable_float(&vertex.normal))
                    .or_try_insert_with(|| -> Result<_> {
                        write_floats(&mut normal_data, &vertex.normal)?;
                        Ok(normal_counter.next())
                    })?;
                let lightmap_coord_index = *lightmap_coord_indices
                    .entry(hashable_float(&vertex.lightmap_coord))
                    .or_try_insert_with(|| -> Result<_> {
                        write_floats(&mut lightmap_coord_data, &vertex.lightmap_coord)?;
                        Ok(lightmap_coord_counter.next())
                    })?;
                let texture_coord_index = *texture_coord_indices
                    .entry(hashable_float(&vertex.texture_coord))
                    .or_try_insert_with(|| -> Result<_> {
                        write_floats(&mut texture_coord_data, &vertex.texture_coord)?;
                        Ok(texture_coord_counter.next())
                    })?;

                if first_position_index.is_none() {
                    first_position_index = Some(position_index);
                    first_normal_index = Some(normal_index);
                    first_lightmap_coord_index = Some(lightmap_coord_index);
                    first_texture_coord_index = Some(texture_coord_index);
                }

                if prev_position_index.is_some() {
                    let mut data = [0; 24];
                    let mut w = &mut data[..];
                    w.write_u16::<BigEndian>(first_position_index.unwrap())?;
                    w.write_u16::<BigEndian>(first_normal_index.unwrap())?;
                    w.write_u16::<BigEndian>(first_lightmap_coord_index.unwrap())?;
                    w.write_u16::<BigEndian>(first_texture_coord_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_position_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_normal_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_lightmap_coord_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_texture_coord_index.unwrap())?;
                    w.write_u16::<BigEndian>(position_index)?;
                    w.write_u16::<BigEndian>(normal_index)?;
                    w.write_u16::<BigEndian>(lightmap_coord_index)?;
                    w.write_u16::<BigEndian>(texture_coord_index)?;
                    batch_builder.emit_vertices(3, &data);
                }
                prev_position_index = Some(position_index);
                prev_normal_index = Some(normal_index);
                prev_lightmap_coord_index = Some(lightmap_coord_index);
                prev_texture_coord_index = Some(texture_coord_index);
            }
        }
    }

    // Build all of the display list builders.
    let display_lists_by_cluster_texture_plane = display_lists_by_cluster_texture_plane
        .into_iter()
        .map(|display_lists_by_texture_plane| {
            display_lists_by_texture_plane
                .into_iter()
                .map(|(texture, display_lists_by_plane)| {
                    (
                        texture,
                        display_lists_by_plane
                            .into_iter()
                            .map(|(plane, display_list)| (plane, display_list.build()))
                            .filter(|(_, x)| !x.is_empty())
                            .collect::<BTreeMap<_, _>>(),
                    )
                })
                .filter(|(_, x)| !x.is_empty())
                .collect::<BTreeMap<_, _>>()
        })
        .collect();

    Ok(ProcessedGeometry {
        position_data,
        normal_data,
        lightmap_coord_data,
        texture_coord_data,
        display_lists_by_cluster_texture_plane,
        texture_ids,
        texture_paths,
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

fn write_debug_env_maps(dst_path: &Path) -> Result<()> {
    let mut data = Vec::new();
    for y in 0..256 {
        for x in 0..256 {
            let s = (x as f32 - 127.5) / 128.0;
            let t = (y as f32 - 127.5) / 128.0;
            let s2t2 = s * s + t * t;
            let rz = (1.0 - s2t2) / (s2t2 + 1.0);
            if rz >= -0.1 {
                let rx = 2.0 * s / (s2t2 + 1.0);
                let ry = 2.0 * t / (s2t2 + 1.0);

                data.push(((rx * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
                data.push(((ry * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
                data.push(((rz * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
            } else {
                data.extend_from_slice(&[0, 0, 0]);
            }
        }
    }
    RgbU8Image::new(256, 256, data)
        .write_to_png(dst_path.join("envmap_front.png").to_str().unwrap())?;

    let mut data = Vec::new();
    for y in 0..256 {
        for x in 0..256 {
            let s = (x as f32 - 127.5) / 128.0;
            let t = (y as f32 - 127.5) / 128.0;
            let s2t2 = s * s + t * t;
            let rz = (s2t2 - 1.0) / (s2t2 + 1.0);
            if rz <= 0.1 {
                let rx = 2.0 * s / (s2t2 + 1.0);
                let ry = 2.0 * t / (s2t2 + 1.0);

                data.push(((rx * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
                data.push(((ry * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
                data.push(((rz * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
            } else {
                data.extend_from_slice(&[0, 0, 0]);
            }
        }
    }
    RgbU8Image::new(256, 256, data)
        .write_to_png(dst_path.join("envmap_back.png").to_str().unwrap())?;

    Ok(())
}

fn write_textures(
    dst_path: &Path,
    asset_loader: &AssetLoader,
    texture_paths: &[VpkPath],
) -> Result<()> {
    const GAMECUBE_MEMORY_BUDGET: u32 = 20 * 1024 * 1024;
    for max_dimension in [1024, 512, 256, 128] {
        let mut total_size = 0;
        for path in texture_paths {
            let texture = asset_loader.get_texture(path)?;
            let mut width = texture.width();
            let mut height = texture.height();
            let image_data = texture.data().unwrap();
            assert!(matches!(image_data.format, ImageFormat::Dxt1));
            // Take all mips that fit within the max_dimension.
            let mut accepted_mip = false;
            for _ in &image_data.mips {
                if width <= max_dimension && height <= max_dimension {
                    total_size += width * height / 2;
                    accepted_mip = true;
                }
                width = (width / 2).max(8);
                height = (height / 2).max(8);
            }
            if !accepted_mip {
                bail!(
                    "unable to find a mipmap within max_dimension={} for texture {}",
                    max_dimension,
                    path,
                )
            }
        }

        println!(
            "Textures occupy {} bytes with max_dimension {}",
            total_size, max_dimension
        );

        if total_size > GAMECUBE_MEMORY_BUDGET {
            continue;
        }

        let mut texture_table = BufWriter::new(File::create(dst_path.join("texture_table.dat"))?);
        let mut texture_data = BufWriter::new(File::create(dst_path.join("texture_data.dat"))?);

        for path in texture_paths {
            let texture = asset_loader.get_texture(path)?;
            let image_data = texture.data().unwrap();
            assert!(matches!(image_data.format, ImageFormat::Dxt1));

            // Take all mips that fit within the max_dimension.
            let start_offset = texture_data.stream_position()? as u32;
            let mut mip_width = texture.width();
            let mut mip_height = texture.height();
            let mut mips_written = 0;
            for mip in &image_data.mips {
                if mip_width <= max_dimension && mip_height <= max_dimension {
                    assert!(mip_width >= 4);
                    assert!(mip_height >= 4);
                    assert_eq!(mip.len(), (mip_width * mip_height / 2) as usize);

                    write_gamecube_dxt1(&mut texture_data, mip, mip_width, mip_height)?;
                    mips_written += 1;
                }
                mip_width = (mip_width / 2).max(4);
                mip_height = (mip_height / 2).max(4);
            }
            // Pad to a 32 byte boundary.
            while (texture_data.stream_position()? & 31) != 0 {
                texture_data.write_u8(0)?;
            }
            let end_offset = texture_data.stream_position()? as u32;

            // Write a texture table entry.
            texture_table.write_u16::<BigEndian>(texture.width() as u16)?;
            texture_table.write_u16::<BigEndian>(texture.height() as u16)?;
            texture_table.write_u8(mips_written)?;
            texture_table.write_u8(
                if (texture.flags() & 0x4) != 0 {
                    0x01
                } else {
                    0
                } | if (texture.flags() & 0x8) != 0 {
                    0x02
                } else {
                    0
                },
            )?;
            texture_table.write_u16::<BigEndian>(0)?;
            texture_table.write_u32::<BigEndian>(start_offset)?;
            texture_table.write_u32::<BigEndian>(end_offset)?;
        }

        texture_table.flush()?;
        texture_data.flush()?;

        return Ok(());
    }
    bail!("Unable to fit textures within the memory budget.");
}

fn write_gamecube_dxt1<W: Write + Seek>(
    dst: &mut W,
    src: &[u8],
    width: u32,
    height: u32,
) -> Result<()> {
    for coarse_y in 0..(height / 8).max(1) {
        for coarse_x in 0..(width / 8).max(1) {
            for fine_y in 0..2 {
                for fine_x in 0..2 {
                    let block_index = (width / 4) * (2 * coarse_y + fine_y) + 2 * coarse_x + fine_x;
                    let block_offset = 8 * block_index as usize;
                    if block_offset < src.len() {
                        let block = &src[block_offset..block_offset + 8];
                        dst.write_u16::<BigEndian>(
                            (&block[..]).read_u16::<LittleEndian>().unwrap(),
                        )?;
                        dst.write_u16::<BigEndian>(
                            (&block[2..]).read_u16::<LittleEndian>().unwrap(),
                        )?;
                        let reverse_two_bit_groups = |x: u32| {
                            let x = ((x & 0x0000ffff) << 16) | ((x & 0xffff0000) >> 16);
                            let x = ((x & 0x00ff00ff) << 8) | ((x & 0xff00ff00) >> 8);
                            let x = ((x & 0x0f0f0f0f) << 4) | ((x & 0xf0f0f0f0) >> 4);
                            let x = ((x & 0x33333333) << 2) | ((x & 0xcccccccc) >> 2);
                            x
                        };
                        dst.write_u32::<BigEndian>(reverse_two_bit_groups(
                            (&block[4..]).read_u32::<LittleEndian>().unwrap(),
                        ))?;
                    } else {
                        // One of the texture dimensions is 4, but GameCube compressed textures are
                        // organized in 8x8 blocks. The blocks that don't exist in the source image
                        // are filled with zeros.
                        dst.write_all(&[0; 8])?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn write_position_data(dst_path: &Path, position_data: Vec<u8>) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("position_data.dat"))?);
    f.write_all(&position_data)?;
    f.flush()?;
    Ok(())
}

fn write_normal_data(dst_path: &Path, normal_data: Vec<u8>) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("normal_data.dat"))?);
    f.write_all(&normal_data)?;
    f.flush()?;
    Ok(())
}

fn write_lightmap_coord_data(dst_path: &Path, lightmap_coord_data: Vec<u8>) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("lightmap_coord_data.dat"))?);
    f.write_all(&lightmap_coord_data)?;
    f.flush()?;
    Ok(())
}

fn write_texture_coord_data(dst_path: &Path, texture_coord_data: Vec<u8>) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("texture_coord_data.dat"))?);
    f.write_all(&texture_coord_data)?;
    f.flush()?;
    Ok(())
}

fn write_display_lists(
    bsp: Bsp,
    dst_path: &Path,
    texture_ids: HashMap<VpkPath, u16>,
    display_lists_by_cluster_texture_plane: Vec<BTreeMap<VpkPath, BTreeMap<u16, Vec<u8>>>>,
) -> Result<()> {
    // struct DisplayListsByClusterTexturePlaneEntry {
    //     by_texture_plane_start_index: u32,
    //     by_texture_plane_end_index: u32,
    // }
    //
    const DISPLAY_LISTS_BY_TEXTURE_PLANE_SIZE: u64 = 12;
    // struct DisplayListsByTexturePlaneEntry {
    //     texture_index: u32,
    //     by_plane_start_index: u32,
    //     by_plane_end_index: u32,
    // }
    //
    const DISPLAY_LISTS_BY_PLANE_SIZE: u64 = 156;
    // struct DisplayListsByPlaneEntry {
    //     plane_index: u16,
    //     _padding: u16,
    //     reflect_front_paraboloid: [[f32; 4]; 3],
    //     reflect_back_paraboloid: [[f32; 4]; 3],
    //     reflect_paraboloid_z: [[f32; 4]; 3],
    //     display_list_start_offset: u32,
    //     display_list_end_offset: u32,
    // }

    let mut file1 = BufWriter::new(File::create(
        dst_path.join("display_lists_by_cluster_texture_plane.dat"),
    )?);
    let mut file2 = RecordWriter::new(
        BufWriter::new(File::create(
            dst_path.join("display_lists_by_texture_plane.dat"),
        )?),
        DISPLAY_LISTS_BY_TEXTURE_PLANE_SIZE,
    );
    let mut file3 = RecordWriter::new(
        BufWriter::new(File::create(dst_path.join("display_lists_by_plane.dat"))?),
        DISPLAY_LISTS_BY_PLANE_SIZE,
    );
    let mut file4 = BufWriter::new(File::create(dst_path.join("display_lists.dat"))?);

    for diplay_lists_by_texture_plane in &display_lists_by_cluster_texture_plane {
        // Emit part of a DisplayListsByClusterTexturePlaneEntry to the first index.
        file1.write_u32::<BigEndian>(file2.index()? as u32)?;

        for (path, display_lists_by_plane) in diplay_lists_by_texture_plane {
            // Emit part of a DisplayListsByTexturePlaneEntry to the second index.
            let texture_index = texture_ids[path] as u32;
            file2.write_u32::<BigEndian>(texture_index)?;
            file2.write_u32::<BigEndian>(file3.index()? as u32)?;

            for (&plane_index, display_list) in display_lists_by_plane {
                let plane = &bsp.planes()[plane_index as usize];
                let normal = make_vec3(&plane.normal);
                let reflect = reflect(&Mat4::identity(), &normal);
                let scale_and_bias = Mat3::from_rows(&[
                    vec3(0.5, 0.0, 0.5).transpose(),
                    vec3(0.0, 0.5, 0.5).transpose(),
                    vec3(0.0, 0.0, 1.0).transpose(),
                ]);
                let reflect_front_paraboloid = scale_and_bias
                    * Mat3x4::from_rows(&[
                        vec4(1.0, 0.0, 0.0, 0.0).transpose(),
                        vec4(0.0, 1.0, 0.0, 0.0).transpose(),
                        vec4(0.0, 0.0, 1.0, 1.0).transpose(),
                    ])
                    * reflect;
                let reflect_back_paraboloid = scale_and_bias
                    * Mat3x4::from_rows(&[
                        vec4(1.0, 0.0, 0.0, 0.0).transpose(),
                        vec4(0.0, 1.0, 0.0, 0.0).transpose(),
                        vec4(0.0, 0.0, -1.0, 1.0).transpose(),
                    ])
                    * reflect;
                let reflect_paraboloid_z = scale_and_bias
                    * Mat3x4::from_rows(&[
                        vec4(0.0, 0.0, 1.0, 0.0).transpose(),
                        vec4(0.0, 0.0, 0.0, 0.0).transpose(),
                        vec4(0.0, 0.0, 0.0, 1.0).transpose(),
                    ])
                    * reflect;

                // Emit part of a DisplayListsByPlaneEntry to the third index.
                file3.write_u16::<BigEndian>(plane_index)?;
                file3.write_u16::<BigEndian>(0)?;
                for row in 0..3 {
                    for col in 0..4 {
                        file3.write_f32::<BigEndian>(reflect_front_paraboloid[(row, col)])?;
                    }
                }
                for row in 0..3 {
                    for col in 0..4 {
                        file3.write_f32::<BigEndian>(reflect_back_paraboloid[(row, col)])?;
                    }
                }
                for row in 0..3 {
                    for col in 0..4 {
                        file3.write_f32::<BigEndian>(reflect_paraboloid_z[(row, col)])?;
                    }
                }
                file3.write_u32::<BigEndian>(file4.stream_position()? as u32)?;

                // Write the display list data to the fourth file.
                file4.write_all(display_list)?;

                // Finish the DisplayListsByPlaneEntry in the third index.
                file3.write_u32::<BigEndian>(file4.stream_position()? as u32)?;
            }

            // Finish the DisplayListsByTexturePlaneEntry in the second index.
            file2.write_u32::<BigEndian>(file3.index()? as u32)?;
        }

        // Finish the DisplayListsByClusterTexturePlaneEntry in the first index.
        file1.write_u32::<BigEndian>(file2.index()? as u32)?;
    }

    file1.flush()?;
    file2.flush()?;
    file3.flush()?;
    file4.flush()?;
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
