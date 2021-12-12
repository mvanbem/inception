use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Seek, Write};
use std::path::Path;
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::{clap_app, crate_authors, crate_description, crate_version};
use memmap::Mmap;
use nalgebra_glm::{mat3_to_mat4, vec2, vec3, vec4, Mat3, Mat3x4, Mat4, Vec3};
use num_traits::PrimInt;
use source_reader::asset::vmt::{LightmappedGeneric, Shader, Vmt};
use source_reader::asset::vtf::{ImageFormat, Vtf};
use source_reader::asset::AssetLoader;
use source_reader::bsp::Bsp;
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::{FallbackFileLoader, FileLoader};
use source_reader::geometry::convert_vertex;
use source_reader::lightmap::{build_lightmaps, ClusterLightmap};
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;

use crate::counter::Counter;
use crate::display_list::DisplayListBuilder;
use crate::record_writer::RecordWriter;
use crate::write_big_endian::WriteBigEndian;

mod counter;
mod display_list;
mod record_writer;
mod write_big_endian;

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

    let cluster_lightmaps = build_lightmaps(bsp)?;
    let map_geometry = process_geometry(bsp, &cluster_lightmaps, &asset_loader)?;

    let dst_path = Path::new(matches.value_of("dst").unwrap_or("."));
    create_dir_all(dst_path)?;

    let texture_ids = write_textures(dst_path, &asset_loader, &cluster_lightmaps, &map_geometry)?;
    write_position_data(dst_path, &map_geometry.position_data)?;
    write_normal_data(dst_path, &map_geometry.normal_data)?;
    write_lightmap_coord_data(dst_path, &map_geometry.lightmap_coord_data)?;
    write_texture_coord_data(dst_path, &map_geometry.texture_coord_data)?;

    write_geometry(dst_path, &map_geometry, &texture_ids)?;
    write_bsp_nodes(dst_path, bsp)?;
    write_bsp_leaves(dst_path, bsp)?;
    write_vis(dst_path, bsp)?;

    Ok(())
}

struct MapGeometry {
    position_data: Vec<u8>,
    normal_data: Vec<u8>,
    lightmap_coord_data: Vec<u8>,
    texture_coord_data: Vec<u8>,
    clusters: Vec<ClusterGeometry>,
    material_paths: Vec<VpkPath>,
}

struct AttributeBuilder<Value, Index> {
    indices: HashMap<Value, Index>,
    counter: Counter<Index>,
    data: Vec<u8>,
}

impl<Value: Copy + Eq + Hash + WriteBigEndian, Index: PrimInt> AttributeBuilder<Value, Index> {
    pub fn new() -> Self {
        Self {
            indices: HashMap::new(),
            counter: Counter::new(),
            data: Vec::new(),
        }
    }

    pub fn add_vertex(&mut self, value: Value) -> Index {
        *self.indices.entry(value).or_insert_with(|| {
            value.write_big_endian_to(&mut self.data).unwrap();
            self.counter.next()
        })
    }

    pub fn build(self) -> Vec<u8> {
        self.data
    }
}

struct PolygonBuilder<'a, Vertex> {
    first_vertex: Option<Vertex>,
    prev_vertex: Option<Vertex>,
    display_list: &'a mut DisplayListBuilder,
}

impl<'a, Vertex: Copy + WriteBigEndian> PolygonBuilder<'a, Vertex> {
    pub fn new(display_list: &'a mut DisplayListBuilder) -> Self {
        Self {
            first_vertex: None,
            prev_vertex: None,
            display_list,
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) -> Result<()> {
        if self.first_vertex.is_none() {
            self.first_vertex = Some(vertex);
        }

        if let (Some(first_vertex), Some(prev_vertex)) = (self.first_vertex, self.prev_vertex) {
            let mut data = Vec::with_capacity(3 * Vertex::SIZE);
            first_vertex.write_big_endian_to(&mut data)?;
            prev_vertex.write_big_endian_to(&mut data)?;
            vertex.write_big_endian_to(&mut data)?;
            self.display_list.emit_vertices(3, &data);
        }
        self.prev_vertex = Some(vertex);
        Ok(())
    }
}

struct ClusterGeometry {
    display_lists_by_pass_batch: BTreeMap<(Pass, Batch), Vec<u8>>,
}

#[derive(Default)]
struct ClusterGeometryBuilder {
    display_lists_by_pass_batch: BTreeMap<(Pass, Batch), DisplayListBuilder>,
}

impl ClusterGeometryBuilder {
    pub fn display_list_builder(&mut self, pass: Pass, batch: Batch) -> &mut DisplayListBuilder {
        self.display_lists_by_pass_batch
            .entry((pass, batch))
            .or_insert_with(|| DisplayListBuilder::new(DisplayListBuilder::TRIANGLES))
    }

    pub fn build(self) -> ClusterGeometry {
        ClusterGeometry {
            display_lists_by_pass_batch: self
                .display_lists_by_pass_batch
                .into_iter()
                .map(|(key, display_list)| (key, display_list.build()))
                .filter(|(_, display_list)| !display_list.is_empty())
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Pass {
    LightmappedGeneric { alpha: PassAlpha, env_map: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PassAlpha {
    Opaque,
    AlphaTest,
    AlphaBlend,
}

impl Pass {
    fn from_material(material: &Vmt) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                env_map,
                translucent,
                ..
            }) => Self::LightmappedGeneric {
                alpha: match (*alpha_test, *translucent) {
                    (false, false) => PassAlpha::Opaque,
                    (false, true) => PassAlpha::AlphaBlend,
                    (true, false) => PassAlpha::AlphaTest,
                    (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                },
                env_map: env_map.is_some(),
            },
            Shader::Unsupported => panic!(),
        }
    }

    fn as_mode(self) -> u8 {
        match self {
            Pass::LightmappedGeneric {
                alpha: PassAlpha::Opaque | PassAlpha::AlphaTest,
                env_map: false,
            } => 0,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::Opaque | PassAlpha::AlphaTest,
                env_map: true,
            } => 1,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                env_map: false,
            } => 0,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                env_map: true,
            } => 1,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Batch {
    base_texture_path: VpkPath,
    alpha: BatchAlpha,
    env_map: Option<BatchEnvMap>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BatchAlpha {
    Opaque,
    AlphaTest { threshold: u8 },
    AlphaBlend,
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct BatchEnvMap {
    plane: [FloatByBits; 3],
    env_map_path: VpkPath,
    env_map_tint: [u8; 3],
}

impl Batch {
    fn from_material_plane(material: &Vmt, plane: [FloatByBits; 3]) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                alpha_test_reference,
                base_texture,
                env_map,
                env_map_tint,
                translucent,
                ..
            }) => {
                let env_map_tint = env_map_tint.unwrap_or(vec3(1.0, 1.0, 1.0));
                Self {
                    base_texture_path: base_texture.path().clone(),
                    alpha: match (*alpha_test, *translucent) {
                        (false, false) => BatchAlpha::Opaque,
                        (false, true) => BatchAlpha::AlphaBlend,
                        (true, false) => BatchAlpha::AlphaTest {
                            threshold: ((alpha_test_reference * 255.0).clamp(0.0, 255.0) + 0.5)
                                as u8,
                        },
                        (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                    },
                    env_map: env_map.as_ref().map(|env_map| BatchEnvMap {
                        plane,
                        env_map_path: env_map.path().clone(),
                        env_map_tint: [
                            ((env_map_tint[0] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                            ((env_map_tint[1] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                            ((env_map_tint[2] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                        ],
                    }),
                }
            }
            Shader::Unsupported => panic!(),
        }
    }
}

fn process_geometry(
    bsp: Bsp,
    cluster_lightmaps: &HashMap<i16, ClusterLightmap>,
    asset_loader: &AssetLoader,
) -> Result<MapGeometry> {
    let mut positions = AttributeBuilder::new();
    let mut normals = AttributeBuilder::new();
    let mut lightmap_coords = AttributeBuilder::new();
    let mut texture_coords = AttributeBuilder::new();
    let mut clusters: Vec<ClusterGeometryBuilder> = Vec::new();
    let mut material_paths = Vec::new();
    let mut material_path_set = HashSet::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        if leaf.cluster == -1 {
            // Leaf is not potentially visible from anywhere.
            continue;
        }
        if clusters.len() < (leaf.cluster as usize + 1) {
            clusters.resize_with(leaf.cluster as usize + 1, Default::default);
        }
        let cluster_builder = &mut clusters[leaf.cluster as usize];
        let cluster_lightmap = &cluster_lightmaps[&leaf.cluster];

        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs == -1 || face.tex_info == -1 {
                // Not a textured lightmapped surface.
                continue;
            }

            let lightmap_metadata = &cluster_lightmap.metadata_by_data_offset[&face.light_ofs];
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
            let base_texture_size = match material.shader() {
                Shader::LightmappedGeneric(LightmappedGeneric { base_texture, .. }) => {
                    [base_texture.width() as f32, base_texture.height() as f32]
                }
                _ => continue,
            };
            if !material_path_set.contains(material.path()) {
                material_path_set.insert(material.path().clone());
                material_paths.push(material.path().clone());
            }
            let plane = hashable_float(&bsp.planes()[face.plane_num as usize].normal);
            let mut polygon_builder = PolygonBuilder::new(cluster_builder.display_list_builder(
                Pass::from_material(&*material),
                Batch::from_material_plane(&*material, plane),
            ));

            for vertex_index in bsp.iter_vertex_indices_from_face(face) {
                let mut vertex = convert_vertex(
                    bsp,
                    &cluster_lightmap.image,
                    lightmap_metadata,
                    face,
                    tex_info,
                    vertex_index,
                );
                vertex.texture_coord = [
                    vertex.texture_coord[0] / base_texture_size[0],
                    vertex.texture_coord[1] / base_texture_size[1],
                ];

                let position_index: u16 = positions.add_vertex(hashable_float(&vertex.position));
                let normal_index: u16 = normals.add_vertex(quantize_normal(vertex.normal));
                let lightmap_coord_index: u16 =
                    lightmap_coords.add_vertex(quantize_lightmap_coord(vertex.lightmap_coord));
                let texture_coord_index: u16 =
                    texture_coords.add_vertex(quantize_texture_coord(vertex.texture_coord));

                polygon_builder.add_vertex((
                    position_index,
                    normal_index,
                    lightmap_coord_index,
                    texture_coord_index,
                ))?;
            }
        }
    }

    Ok(MapGeometry {
        position_data: positions.build(),
        normal_data: normals.build(),
        lightmap_coord_data: lightmap_coords.build(),
        texture_coord_data: texture_coords.build(),
        clusters: clusters
            .into_iter()
            .map(ClusterGeometryBuilder::build)
            .collect(),
        material_paths,
    })
}

fn quantize_normal(normal: [f32; 3]) -> [u8; 3] {
    let mut result = [0; 3];
    for index in 0..3 {
        result[index] = ((normal[index] * 64.0).clamp(-64.0, 64.0) + 0.5) as i8 as u8;
    }
    result
}

fn quantize_lightmap_coord(coord: [f32; 2]) -> [u16; 2] {
    let mut result = [0; 2];
    for index in 0..2 {
        result[index] = (coord[index] * 32768.0).clamp(0.0, 65535.0) as u16;
    }
    result
}

fn quantize_texture_coord(coord: [f32; 2]) -> [i16; 2] {
    let mut result = [0; 2];
    for index in 0..2 {
        result[index] = (coord[index] * 256.0).round().clamp(-32768.0, 32767.0) as i16;
    }
    result
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

#[derive(Clone, Copy, Debug)]
pub struct FloatByBits(f32);

impl Eq for FloatByBits {}

impl Hash for FloatByBits {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for FloatByBits {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits().eq(&other.0.to_bits())
    }
}

impl Ord for FloatByBits {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bits().cmp(&other.0.to_bits())
    }
}

impl PartialOrd for FloatByBits {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.to_bits().partial_cmp(&other.0.to_bits())
    }
}

fn hashable_float<const N: usize>(array: &[f32; N]) -> [FloatByBits; N] {
    let mut result = [FloatByBits(0.0); N];
    for index in 0..N {
        result[index] = FloatByBits(array[index]);
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum TextureType {
    Image,
    Envmap { plane: [FloatByBits; 3] },
    Lightmap { cluster: i16 },
}

fn write_textures(
    dst_path: &Path,
    asset_loader: &AssetLoader,
    cluster_lightmaps: &HashMap<i16, ClusterLightmap>,
    map_geometry: &MapGeometry,
) -> Result<HashMap<(VpkPath, Option<[FloatByBits; 3]>), u16>> {
    let mut planes_by_env_map: HashMap<VpkPath, HashSet<[FloatByBits; 3]>> = Default::default();
    for cluster_geometry in &map_geometry.clusters {
        for (_, batch) in cluster_geometry.display_lists_by_pass_batch.keys() {
            if let Some(env_map) = batch.env_map.as_ref() {
                planes_by_env_map
                    .entry(env_map.env_map_path.clone())
                    .or_default()
                    .insert(env_map.plane);
            }
        }
    }

    let mut textures = Vec::new();
    let mut texture_ids = HashMap::new();
    for material_path in &map_geometry.material_paths {
        match asset_loader.get_material(material_path)?.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                base_texture,
                env_map,
                ..
            }) => {
                texture_ids
                    .entry((base_texture.path().clone(), None))
                    .or_insert_with(|| {
                        let value = textures.len() as u16;
                        textures.push((TextureType::Image, Rc::clone(base_texture)));
                        value
                    });
                if let Some(env_map) = env_map.as_ref() {
                    for plane in planes_by_env_map[env_map.path()].iter().copied() {
                        texture_ids
                            .entry((env_map.path().clone(), Some(plane)))
                            .or_insert_with(|| {
                                let value = textures.len() as u16;
                                textures.push((TextureType::Envmap { plane }, Rc::clone(env_map)));
                                value
                            });
                    }
                }
            }
            Shader::Unsupported => panic!(),
        };
    }
    for (cluster, cluster_lightmaps) in cluster_lightmaps {
        let path =
            VpkPath::new_with_prefix_and_extension(&format!("{}", *cluster), "lightmap", "vtf");
        let texture_id = textures.len() as u16;
        textures.push((
            TextureType::Lightmap { cluster: *cluster },
            Rc::new(Vtf::new(path.clone(), &cluster_lightmaps.image)),
        ));
        texture_ids.insert((path, None), texture_id);
    }

    const GAMECUBE_MEMORY_BUDGET: u32 = 16 * 1024 * 1024;
    for dimension_divisor in [1, 2, 4, 8, 16, 32] {
        let mut total_size = 0;
        for (type_, texture) in &textures {
            let mut width = texture.width();
            let mut height = texture.height();
            if texture.data().is_none() {
                bail!("no texture data for {}", texture.path());
            }
            let image_data = texture.data().unwrap();
            match type_ {
                TextureType::Image => {
                    // Take all mips that fit within the max_dimension.
                    let mut accepted_mip = false;
                    let max_width = (width / dimension_divisor).max(8);
                    let max_height = (height / dimension_divisor).max(8);
                    for _ in &image_data.mips {
                        if width <= max_width && height <= max_height {
                            match image_data.format {
                                ImageFormat::Dxt1 => total_size += width * height / 2,
                                ImageFormat::Rgb8 | ImageFormat::Rgba8 => {
                                    total_size += width * height * 4
                                }
                            }
                            accepted_mip = true;
                        }
                        width = width / 2;
                        height = height / 2;
                        if width < 8 || height < 8 {
                            break;
                        }
                    }
                    if !accepted_mip {
                        bail!(
                            "unable to find a mipmap within max_size={}x{} for texture {}",
                            max_width,
                            max_height,
                            texture.path(),
                        );
                    }
                }
                TextureType::Envmap { .. } => {
                    // Emit a sphere map at double the cube face dimension.
                    let width = 2 * width / dimension_divisor;
                    let height = 2 * height / dimension_divisor;

                    // Use RGBA8, making this 32 bpp.
                    // TODO: Use DXT1.
                    total_size += width * height * 4;
                }
                TextureType::Lightmap { .. } => {
                    // Use RGBA8, making this 32 bpp.
                    // TODO: Use DXT1.
                    total_size += width * height * 4;
                }
            }
        }

        println!(
            "Textures occupy {} bytes with dimension_divisor {}",
            total_size, dimension_divisor,
        );

        if total_size > GAMECUBE_MEMORY_BUDGET {
            continue;
        }

        let mut texture_table = BufWriter::new(File::create(dst_path.join("texture_table.dat"))?);
        let mut texture_data = BufWriter::new(File::create(dst_path.join("texture_data.dat"))?);

        total_size = 0;
        for (type_, texture) in &textures {
            let image_data = texture.data().unwrap();
            match type_ {
                TextureType::Image => {
                    assert_eq!(image_data.layer_count, 1);

                    // Take all mips that fit within the max_dimension.
                    let max_mip_width = (texture.width() / dimension_divisor).max(8);
                    let max_mip_height = (texture.height() / dimension_divisor).max(8);
                    let start_offset = texture_data.stream_position()? as u32;
                    let mut mip_width = texture.width();
                    let mut mip_height = texture.height();
                    let mut mips_written = 0;
                    for mip in &image_data.mips {
                        if mip_width <= max_mip_width && mip_height <= max_mip_height {
                            match image_data.format {
                                ImageFormat::Dxt1 => {
                                    // Already compressed. Just have to adapt the bit and byte order.
                                    assert_eq!(mip[0].len(), (mip_width * mip_height / 2) as usize);
                                    write_gamecube_dxt1(
                                        &mut texture_data,
                                        &mip[0],
                                        mip_width,
                                        mip_height,
                                    )?;
                                }
                                ImageFormat::Rgba8 => {
                                    write_gamecube_rgba8(
                                        &mut texture_data,
                                        &mip[0],
                                        mip_width,
                                        mip_height,
                                    )?;
                                }
                                _ => bail!(
                                    "unexpected image format for TextureType::Image: {:?}",
                                    image_data.format,
                                ),
                            }
                            mips_written += 1;
                        }
                        mip_width = mip_width / 2;
                        mip_height = mip_height / 2;
                        if mip_width < 8 || mip_height < 8 {
                            break;
                        }
                    }
                    // Pad to a 32 byte boundary.
                    while (texture_data.stream_position()? & 31) != 0 {
                        texture_data.write_u8(0)?;
                    }
                    let end_offset = texture_data.stream_position()? as u32;

                    // Write a texture table entry.
                    texture_table
                        .write_u16::<BigEndian>((texture.width() / dimension_divisor) as u16)?;
                    texture_table
                        .write_u16::<BigEndian>((texture.height() / dimension_divisor) as u16)?;
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
                    texture_table.write_u8(match image_data.format {
                        ImageFormat::Dxt1 => 0xe,  // GX_TF_CMPR
                        ImageFormat::Rgba8 => 0x6, // GX_TF_RGBA8
                        _ => bail!(
                            "unexpected image format for TextureType::Image: {:?}",
                            image_data.format,
                        ),
                    })?;
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += end_offset - start_offset;
                }
                TextureType::Envmap { plane } => {
                    // Emit a sphere map at the cube face dimension.
                    // Use RGBA8, making this 32 bpp.
                    // TODO: Use DXT1.
                    assert_eq!(texture.width(), texture.height());
                    let cube_size = texture.width() as usize;
                    let sphere_width = 2 * cube_size / dimension_divisor as usize;
                    let sphere_height = 2 * cube_size / dimension_divisor as usize;
                    let mut pixels = Vec::with_capacity(sphere_width * sphere_height);

                    // Decode the six cube map faces to flat RGBA8 buffers.
                    let mut faces = Vec::new();
                    match image_data.format {
                        ImageFormat::Dxt1 => {
                            let encoded_faces = &image_data.mips[0];
                            assert!(encoded_faces.len() == 6 || encoded_faces.len() == 7);
                            for face_index in 0..6 {
                                let encoded_face = &encoded_faces[face_index];
                                assert_eq!(encoded_face.len(), cube_size * cube_size / 2);
                                let mut face = Vec::with_capacity(cube_size * cube_size * 4);
                                read_native_dxt1(&mut face, encoded_face, cube_size, cube_size)?;
                                faces.push(face);
                            }
                        }
                        _ => bail!("unexpected cube map format: {:?}", image_data.format),
                    }
                    let sample = |v: Vec3| -> [u8; 4] {
                        let (face, s, t) = if v[0].abs() >= v[1].abs() && v[0].abs() >= v[2].abs() {
                            if v[0] > 0.0 {
                                // X+
                                (1, -v[2] / v[0] * 0.5 + 0.5, v[1] / v[0] * 0.5 + 0.5)
                            } else {
                                // X-
                                (0, -v[2] / v[0] * 0.5 + 0.5, -v[1] / v[0] * 0.5 + 0.5)
                            }
                        } else if v[1].abs() >= v[2].abs() {
                            if v[1] > 0.0 {
                                // Y+
                                (3, -v[0] / v[1] * 0.5 + 0.5, v[2] / v[1] * 0.5 + 0.5)
                            } else {
                                // Y-
                                (2, v[0] / v[1] * 0.5 + 0.5, v[2] / v[1] * 0.5 + 0.5)
                            }
                        } else {
                            if v[2] > 0.0 {
                                // Z+
                                (5, v[0] / v[2] * 0.5 + 0.5, v[1] / v[2] * 0.5 + 0.5)
                            } else {
                                // Z-
                                (4, v[0] / v[2] * 0.5 + 0.5, -v[1] / v[2] * 0.5 + 0.5)
                            }
                        };

                        let x = (s * cube_size as f32).clamp(0.0, cube_size as f32);
                        let y = (t * cube_size as f32).clamp(0.0, cube_size as f32);
                        let x0 = (x as usize).min(cube_size - 1);
                        let y0 = (y as usize).min(cube_size - 1);
                        let x1 = (x0 + 1).min(cube_size - 1);
                        let y1 = (y0 + 1).min(cube_size - 1);
                        let xf = x.fract();
                        let yf = y.fract();

                        let offset = 4 * (cube_size * y0 + x0);
                        let sample0: [u8; 4] = faces[face][offset..offset + 4].try_into().unwrap();
                        let offset = 4 * (cube_size * y0 + x1);
                        let sample1: [u8; 4] = faces[face][offset..offset + 4].try_into().unwrap();
                        let offset = 4 * (cube_size * y1 + x0);
                        let sample2: [u8; 4] = faces[face][offset..offset + 4].try_into().unwrap();
                        let offset = 4 * (cube_size * y1 + x1);
                        let sample3: [u8; 4] = faces[face][offset..offset + 4].try_into().unwrap();

                        let lerp = |a, b, t| (1.0 - t) * a + t * b;
                        let quantize = |x: f32| (x + 0.5).clamp(0.0, 255.0) as u8;
                        let filter = |s0, s1, s2, s3| {
                            quantize(lerp(
                                lerp(s0 as f32, s1 as f32, xf),
                                lerp(s2 as f32, s3 as f32, xf),
                                yf,
                            ))
                        };

                        [
                            filter(sample0[0], sample1[0], sample2[0], sample3[0]),
                            filter(sample0[1], sample1[1], sample2[1], sample3[1]),
                            filter(sample0[2], sample1[2], sample2[2], sample3[2]),
                            filter(sample0[3], sample1[3], sample2[3], sample3[3]),
                        ]
                    };

                    // Sample the decoded cube map to build the sphere map.
                    let normal = vec3(plane[0].0, plane[1].0, plane[2].0);
                    let (s, t) = build_local_space(&normal);
                    let mut png_pixels = Vec::with_capacity(sphere_width * sphere_height * 3);
                    for y in 0..sphere_height {
                        for x in 0..sphere_width {
                            let tex_s = (x as f32 + 0.5) / sphere_width as f32 * 2.0 - 1.0;
                            let tex_t = (y as f32 + 0.5) / sphere_height as f32 * 2.0 - 1.0;
                            let tex_zsqr = 1.0 - tex_s * tex_s - tex_t * tex_t;
                            let (tex_s, tex_t, tex_zsqr) = if tex_zsqr >= 0.0 {
                                (tex_s, tex_t, tex_zsqr)
                            } else {
                                let st = vec2(tex_s, tex_t).normalize();
                                (st[0], st[1], 0.0)
                            };

                            let incident = vec3(0.0, 0.0, 1.0);
                            let sphere_normal = vec3(tex_s, tex_t, tex_zsqr.sqrt());
                            let vec =
                                incident - sphere_normal * (2.0 * incident.dot(&sphere_normal));
                            let world_vec = Mat3::from_columns(&[s, t, normal]) * vec;

                            let rgba = sample(world_vec);
                            pixels.extend_from_slice(&rgba);
                            png_pixels.extend_from_slice(&rgba[..3]);
                        }
                    }

                    let start_offset = texture_data.stream_position()? as u32;
                    write_gamecube_rgba8(
                        &mut texture_data,
                        &pixels,
                        sphere_width as u32,
                        sphere_height as u32,
                    )?;
                    let end_offset = texture_data.stream_position()? as u32;

                    // Write a texture table entry.
                    texture_table.write_u16::<BigEndian>(sphere_width as u16)?;
                    texture_table.write_u16::<BigEndian>(sphere_height as u16)?;
                    texture_table.write_u8(1)?; // mip count
                    texture_table.write_u8(0x3)?; // flags: CLAMP_S | CLAMP_T
                    texture_table.write_u8(0x6)?; // GX_TF_RGBA8
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += end_offset - start_offset;
                }
                TextureType::Lightmap { .. } => {
                    assert_eq!(image_data.format, ImageFormat::Rgb8);

                    let mut rgb_data = image_data.mips[0][0].as_slice();
                    let mut rgba_data = Vec::with_capacity(
                        4 * texture.width() as usize * texture.height() as usize,
                    );
                    for _ in 0..texture.width() as usize * texture.height() as usize {
                        rgba_data.extend_from_slice(&rgb_data[..3]);
                        rgb_data = &rgb_data[3..];
                        rgba_data.push(255);
                    }

                    let start_offset = texture_data.stream_position()? as u32;
                    write_gamecube_rgba8(
                        &mut texture_data,
                        &rgba_data,
                        texture.width(),
                        texture.height(),
                    )?;
                    let end_offset = texture_data.stream_position()? as u32;

                    // Write a texture table entry.
                    texture_table.write_u16::<BigEndian>(texture.width() as u16)?;
                    texture_table.write_u16::<BigEndian>(texture.height() as u16)?;
                    texture_table.write_u8(1)?; // mip count
                    texture_table.write_u8(0x3)?; // flags: CLAMP_S | CLAMP_T
                    texture_table.write_u8(0x6)?; // GX_TF_RGBA8
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += end_offset - start_offset;
                }
            }
        }

        println!("wrote total size: {} bytes", total_size);

        texture_table.flush()?;
        texture_data.flush()?;

        return Ok(texture_ids);
    }
    bail!("Unable to fit textures within the memory budget.");
}

fn read_native_dxt1<W: Write>(dst: &mut W, src: &[u8], width: usize, height: usize) -> Result<()> {
    let blocks_wide = (width + 3) / 4;
    for y in 0..height {
        let coarse_y = y / 4;
        for x in 0..width {
            let coarse_x = x / 4;
            let dxt1_offset = 8 * (blocks_wide * coarse_y + coarse_x);
            let mut block = &src[dxt1_offset..dxt1_offset + 8];
            let color_a_encoded = block.read_u16::<LittleEndian>()?;
            let color_b_encoded = block.read_u16::<LittleEndian>()?;
            let color_a = decode_rgb565(color_a_encoded);
            let color_b = decode_rgb565(color_b_encoded);
            let colors = if color_a > color_b {
                [
                    color_a,
                    color_b,
                    [
                        ((2 * color_a[0] as u16 + color_b[0] as u16) / 3) as u8,
                        ((2 * color_a[1] as u16 + color_b[1] as u16) / 3) as u8,
                        ((2 * color_a[2] as u16 + color_b[2] as u16) / 3) as u8,
                        ((2 * color_a[3] as u16 + color_b[3] as u16) / 3) as u8,
                    ],
                    [
                        ((color_a[0] as u16 + 2 * color_b[0] as u16) / 3) as u8,
                        ((color_a[1] as u16 + 2 * color_b[1] as u16) / 3) as u8,
                        ((color_a[2] as u16 + 2 * color_b[2] as u16) / 3) as u8,
                        ((color_a[3] as u16 + 2 * color_b[3] as u16) / 3) as u8,
                    ],
                ]
            } else {
                [
                    color_a,
                    color_b,
                    [
                        ((color_a[0] as u16 + color_b[0] as u16) / 2) as u8,
                        ((color_a[1] as u16 + color_b[1] as u16) / 2) as u8,
                        ((color_a[2] as u16 + color_b[2] as u16) / 2) as u8,
                        ((color_a[3] as u16 + color_b[3] as u16) / 2) as u8,
                    ],
                    [0, 0, 0, 0],
                ]
            };
            let color_bit = 2 * (4 * (y % 4) + (x % 4));
            let color_bits = block.read_u32::<LittleEndian>().unwrap();
            let color = colors[((color_bits >> color_bit) & 3) as usize];
            dst.write_all(&color)?;
        }
    }
    Ok(())
}

fn decode_rgb565(encoded: u16) -> [u8; 4] {
    let extend5 = |x| (x << 3) | (x >> 2);
    let extend6 = |x| (x << 2) | (x >> 4);
    [
        extend5(((encoded >> 11) & 0x1f) as u8),
        extend6(((encoded >> 5) & 0x3f) as u8),
        extend5((encoded & 0x1f) as u8),
        255,
    ]
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

fn write_gamecube_rgba8<W: Write + Seek>(
    dst: &mut W,
    src: &[u8],
    width: u32,
    height: u32,
) -> Result<()> {
    for coarse_y in 0..height / 4 {
        for coarse_x in 0..width / 4 {
            let mut block = [0; 64];
            for fine_y in 0..4 {
                for fine_x in 0..4 {
                    let x = 4 * coarse_x + fine_x;
                    let y = 4 * coarse_y + fine_y;
                    let src_offset = 4 * (width * y + x) as usize;
                    let rgba: &[u8; 4] = src[src_offset..src_offset + 4].try_into().unwrap();
                    let dst_offset = 2 * (4 * fine_y + fine_x) as usize;
                    block[dst_offset] = rgba[3];
                    block[dst_offset + 1] = rgba[0];
                    block[dst_offset + 32] = rgba[1];
                    block[dst_offset + 33] = rgba[2];
                }
            }
            dst.write_all(&block)?;
        }
    }
    Ok(())
}

fn write_position_data(dst_path: &Path, position_data: &[u8]) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("position_data.dat"))?);
    f.write_all(&position_data)?;
    f.flush()?;
    Ok(())
}

fn write_normal_data(dst_path: &Path, normal_data: &[u8]) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("normal_data.dat"))?);
    f.write_all(&normal_data)?;
    f.flush()?;
    Ok(())
}

fn write_lightmap_coord_data(dst_path: &Path, lightmap_coord_data: &[u8]) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("lightmap_coord_data.dat"))?);
    f.write_all(&lightmap_coord_data)?;
    f.flush()?;
    Ok(())
}

fn write_texture_coord_data(dst_path: &Path, texture_coord_data: &[u8]) -> Result<()> {
    let mut f = BufWriter::new(File::create(dst_path.join("texture_coord_data.dat"))?);
    f.write_all(&texture_coord_data)?;
    f.flush()?;
    Ok(())
}

mod byte_code {
    use std::ops::Index;

    use crate::BatchAlpha;

    pub fn draw(display_list_start_offset: u32, display_list_end_offset: u32) -> [u32; 2] {
        assert_eq!(display_list_start_offset >> 24, 0);
        [display_list_start_offset, display_list_end_offset]
    }

    pub fn set_plane(texture_matrix: impl Index<(usize, usize), Output = f32>) -> [u32; 13] {
        [
            0x01000000,
            texture_matrix[(0, 0)].to_bits(),
            texture_matrix[(0, 1)].to_bits(),
            texture_matrix[(0, 2)].to_bits(),
            texture_matrix[(0, 3)].to_bits(),
            texture_matrix[(1, 0)].to_bits(),
            texture_matrix[(1, 1)].to_bits(),
            texture_matrix[(1, 2)].to_bits(),
            texture_matrix[(1, 3)].to_bits(),
            texture_matrix[(2, 0)].to_bits(),
            texture_matrix[(2, 1)].to_bits(),
            texture_matrix[(2, 2)].to_bits(),
            texture_matrix[(2, 3)].to_bits(),
        ]
    }

    pub fn set_base_texture(base_texture_index: u16) -> [u32; 1] {
        [0x02000000 | base_texture_index as u32]
    }

    pub fn set_env_map_texture(env_map_texture_index: u16) -> [u32; 1] {
        [0x03000000 | env_map_texture_index as u32]
    }

    pub fn set_env_map_tint(env_map_tint: [u8; 3]) -> [u32; 1] {
        [0x04000000
            | ((env_map_tint[0] as u32) << 16)
            | ((env_map_tint[1] as u32) << 8)
            | env_map_tint[2] as u32]
    }

    pub fn set_alpha(alpha: BatchAlpha) -> [u32; 1] {
        let test: u32 = match alpha {
            BatchAlpha::AlphaTest { .. } => 1,
            _ => 0,
        };
        let threshold: u32 = match alpha {
            BatchAlpha::AlphaTest { threshold } => threshold as u32,
            _ => 0,
        };
        let blend: u32 = match alpha {
            BatchAlpha::AlphaBlend => 1,
            _ => 0,
        };
        [0x05000000 | (test << 16) | (threshold << 8) | blend]
    }

    pub fn set_lightmap_texture(lightmap_texture_index: u16) -> [u32; 1] {
        [0x06000000 | lightmap_texture_index as u32]
    }

    pub fn set_mode(mode: u8) -> [u32; 1] {
        [0xff000000 | mode as u32]
    }
}

fn build_local_space(normal: &Vec3) -> (Vec3, Vec3) {
    // Choose a reference vector that is not nearly aligned with the normal. This is fairly
    // arbitrary and is done only to establish a local coordinate space. Use the Y axis if the
    // normal points mostly along the X axis. Otherwise, choose the X axis.
    let r = if normal[0].abs() >= normal[1].abs() && normal[0].abs() >= normal[2].abs() {
        vec3(0.0, 1.0, 0.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    };
    // Construct s perpendicular to the normal and reference vectors.
    let s = normal.cross(&r).normalize();
    // Construct t perpendicular to the s and normal vectors. The order is chosen so that
    // `s x t = normal`.
    let t = normal.cross(&s).normalize();
    (s, t)
}

fn write_geometry(
    dst_path: &Path,
    map_geometry: &MapGeometry,
    texture_ids: &HashMap<(VpkPath, Option<[FloatByBits; 3]>), u16>,
) -> Result<()> {
    // struct ClusterGeometry {
    //     byte_code_start_index: u32,
    //     byte_code_end_index: u32,
    // }

    let mut table_file = BufWriter::new(File::create(dst_path.join("cluster_geometry_table.dat"))?);
    let mut byte_code_file = RecordWriter::new(
        BufWriter::new(File::create(
            dst_path.join("cluster_geometry_byte_code.dat"),
        )?),
        4,
    );
    let mut display_lists_file = BufWriter::new(File::create(dst_path.join("display_lists.dat"))?);

    for (cluster_index, cluster) in map_geometry.clusters.iter().enumerate() {
        // Emit part of a ClusterGeometry struct to the table.
        table_file.write_u32::<BigEndian>(byte_code_file.index()? as u32)?;

        // Each cluster's data starts with the lightmap that is used for that entire cluster.
        let lightmap_path = VpkPath::new_with_prefix_and_extension(
            &format!("{}", cluster_index),
            "lightmap",
            "vtf",
        );
        let lightmap_texture_index = texture_ids[&(lightmap_path, None)];
        byte_code::set_lightmap_texture(lightmap_texture_index)
            .write_big_endian_to(&mut *byte_code_file)?;

        let mut prev_mode = None;
        let mut prev_base_texture_path = None;
        let mut prev_plane = None;
        let mut prev_env_map_path = None;
        let mut prev_env_map_tint = None;
        let mut prev_alpha = None;
        for ((pass, batch), display_list) in &cluster.display_lists_by_pass_batch {
            let mode = pass.as_mode();
            let plane = batch.env_map.as_ref().map(|env_map| env_map.plane);
            let env_map_path = batch.env_map.as_ref().map(|env_map| &env_map.env_map_path);
            let env_map_tint = batch.env_map.as_ref().map(|env_map| env_map.env_map_tint);

            if prev_mode != Some(mode) {
                prev_mode = Some(mode);

                byte_code::set_mode(mode).write_big_endian_to(&mut *byte_code_file)?;
            }

            if prev_base_texture_path != Some(&batch.base_texture_path) {
                prev_base_texture_path = Some(&batch.base_texture_path);

                let base_texture_index = texture_ids[&(batch.base_texture_path.clone(), None)];
                byte_code::set_base_texture(base_texture_index)
                    .write_big_endian_to(&mut *byte_code_file)?;
            }

            if prev_plane != plane {
                prev_plane = plane;

                if let Some(plane) = plane {
                    let normal = vec3(plane[0].0, plane[1].0, plane[2].0);
                    let (s, t) = build_local_space(&normal);

                    // Map world space vectors to (s, t, normal) local space.
                    let world_to_local = mat3_to_mat4(&Mat3::from_rows(&[
                        s.transpose(),
                        t.transpose(),
                        normal.transpose(),
                    ]));
                    // Map local space vectors to their mirror images relative to the
                    // `z = 0` plane.
                    let local_reflect = Mat4::from_diagonal(&vec4(1.0, 1.0, -1.0, 1.0));
                    // Map normalized vectors in local space to texture coordinates.
                    let local_to_texture = Mat3x4::from_rows(&[
                        vec4(0.5, 0.0, 0.0, 0.5).transpose(),
                        vec4(0.0, 0.5, 0.0, 0.5).transpose(),
                        vec4(0.0, 0.0, 0.0, 1.0).transpose(),
                    ]);

                    // Their product maps world space vectors to reflection texture coordinates.
                    let texture_matrix: Mat3x4 = local_to_texture * local_reflect * world_to_local;

                    byte_code::set_plane(texture_matrix)
                        .write_big_endian_to(&mut *byte_code_file)?;
                }
            }

            if prev_env_map_path != Some(env_map_path) {
                prev_env_map_path = Some(env_map_path);

                if let Some(env_map_path) = env_map_path {
                    let env_map_texture_index =
                        texture_ids[&(env_map_path.clone(), Some(plane.unwrap()))];
                    byte_code::set_env_map_texture(env_map_texture_index)
                        .write_big_endian_to(&mut *byte_code_file)?;
                }
            }

            if prev_env_map_tint != env_map_tint {
                prev_env_map_tint = env_map_tint;

                if let Some(env_map_tint) = env_map_tint {
                    byte_code::set_env_map_tint(env_map_tint)
                        .write_big_endian_to(&mut *byte_code_file)?;
                }
            }

            if prev_alpha != Some(batch.alpha) {
                prev_alpha = Some(batch.alpha);

                byte_code::set_alpha(batch.alpha).write_big_endian_to(&mut *byte_code_file)?;
            }

            let display_list_start_offset = display_lists_file.stream_position()? as u32;
            assert_eq!(display_list_start_offset & 31, 0);
            display_lists_file.write_all(display_list)?;
            let display_list_end_offset = display_lists_file.stream_position()? as u32;
            assert_eq!(display_list_end_offset & 31, 0);

            byte_code::draw(display_list_start_offset, display_list_end_offset)
                .write_big_endian_to(&mut *byte_code_file)?;
        }

        // Finish the ClusterGeometry struct in the table.
        table_file.write_u32::<BigEndian>(byte_code_file.index()? as u32)?;
    }

    table_file.flush()?;
    byte_code_file.flush()?;
    display_lists_file.flush()?;
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
