use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::hash::{Hash, Hasher};
use std::io::{stdout, BufWriter, Seek, Write};
use std::path::Path;
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use byteorder::{BigEndian, WriteBytesExt};
use clap::{clap_app, crate_authors, crate_description, crate_version, ArgMatches};
use memmap::Mmap;
use nalgebra_glm::{mat3_to_mat4, vec2, vec3, vec4, Mat3, Mat3x4, Mat4, Vec3};
use num_traits::PrimInt;
use source_reader::asset::vmt::{LightmappedGeneric, Shader, Vmt};
use source_reader::asset::AssetLoader;
use source_reader::bsp::Bsp;
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::{FallbackFileLoader, FileLoader};
use source_reader::geometry::convert_vertex;
use source_reader::lightmap::{build_lightmaps, ClusterLightmap, LightmapPatch};
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;
use texture_format::{
    AnyTexture, AnyTextureBuf, GxTfCmpr, GxTfRgba8, Rgba8, TextureBuf, TextureFormatExt,
};

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
        (@subcommand pack_map =>
            (about: "packs a single map for use on GC/Wii")
            (@arg MAP: "Map name (default: d1_trainstation_01)")
            (@arg dst: --dst [PATH] "Path to write packed outputs (default: .)")
        )
        (@subcommand cat_material =>
            (about: "prints a material definition to stdout")
            (@arg NAME: ... "Material name (example: tile/tilefloor013a)")
        )
    )
    .get_matches();

    let hl2_base = Path::new(matches.value_of("hl2_base").unwrap());
    match matches.subcommand() {
        ("pack_map", Some(matches)) => pack_map(hl2_base, matches)?,
        ("cat_material", Some(matches)) => cat_material(hl2_base, matches)?,
        (name, _) => bail!("unknown subcommand: {:?}", name),
    }
    Ok(())
}

fn cat_material(hl2_base: &Path, matches: &ArgMatches) -> Result<()> {
    let file_loader = Vpk::new(hl2_base.join("hl2_misc"))?;
    for name in matches.values_of("NAME").unwrap() {
        let path = VpkPath::new_with_prefix_and_extension(name, "materials", "vmt");
        let file = match file_loader.load_file(&path)? {
            Some(data) => data,
            None => bail!("asset not found: {}", path),
        };
        let stdout = stdout();
        let mut stdout = stdout.lock();
        stdout.write_all(&file)?;
        stdout.flush()?;
    }

    Ok(())
}

fn pack_map(hl2_base: &Path, matches: &ArgMatches) -> Result<()> {
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

    let pak_loader = Rc::new(ZipArchiveLoader::new(bsp.pak_file()));
    let material_loader = Rc::new(FallbackFileLoader::new(vec![
        Rc::clone(&pak_loader) as _,
        Rc::new(Vpk::new(hl2_base.join("hl2_misc"))?),
    ]));
    let texture_loader = Rc::new(FallbackFileLoader::new(vec![
        pak_loader,
        Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?),
    ]));
    let asset_loader = AssetLoader::new(material_loader, texture_loader);

    let cluster_lightmaps = build_lightmaps(bsp)?;
    let map_geometry = process_geometry(bsp, &cluster_lightmaps, &asset_loader)?;

    let dst_path = Path::new(matches.value_of("dst").unwrap_or("."));
    create_dir_all(dst_path)?;

    let texture_ids = write_textures(dst_path, &asset_loader, &map_geometry)?;
    write_position_data(dst_path, &map_geometry.position_data)?;
    write_normal_data(dst_path, &map_geometry.normal_data)?;
    write_lightmap_coord_data(dst_path, &map_geometry.lightmap_coord_data)?;
    write_texture_coord_data(dst_path, &map_geometry.texture_coord_data)?;

    write_geometry(dst_path, &map_geometry, &texture_ids)?;
    write_bsp_nodes(dst_path, bsp)?;
    write_bsp_leaves(dst_path, bsp)?;
    write_vis(dst_path, bsp)?;
    write_lightmaps(dst_path, bsp, &cluster_lightmaps)?;

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
    LightmappedGeneric {
        alpha: PassAlpha,
        env_map: Option<PassEnvMap>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PassAlpha {
    Opaque,
    AlphaTest,
    AlphaBlend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct PassEnvMap {
    mask: PassEnvMapMask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PassEnvMapMask {
    None,
    Texture,
    BaseAlpha,
    BumpMapAlpha,
}

impl Pass {
    fn from_material(material: &Vmt) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                base_alpha_env_map_mask,
                env_map,
                env_map_mask,
                normal_map_alpha_env_map_mask,
                translucent,
                ..
            }) => Self::LightmappedGeneric {
                alpha: match (*alpha_test, *translucent) {
                    (false, false) => PassAlpha::Opaque,
                    (false, true) => PassAlpha::AlphaBlend,
                    (true, false) => PassAlpha::AlphaTest,
                    (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                },
                env_map: env_map.as_ref().map(|_| PassEnvMap {
                    mask: match (
                        env_map_mask.is_some(),
                        base_alpha_env_map_mask,
                        normal_map_alpha_env_map_mask,
                    ) {
                        (false, false, false) => PassEnvMapMask::None,
                        (true, false, false) => PassEnvMapMask::Texture,
                        (false, true, false) => PassEnvMapMask::BaseAlpha,
                        (false, false, true) => PassEnvMapMask::BumpMapAlpha,
                        _ => panic!("bad env map mask combination: base_alpha_env_map_mask={}, normal_map_alpha_env_map_mask={}, env_map_mask={:?}",
                            base_alpha_env_map_mask,
                            normal_map_alpha_env_map_mask,
                            env_map_mask.as_ref().map(|vtf| vtf.path().as_canonical_path()),
                        ),
                    },
                }),
            },
            Shader::Unsupported => panic!(),
        }
    }

    fn as_mode(self) -> u8 {
        match self {
            Pass::LightmappedGeneric {
                alpha: _,
                env_map: None,
            } => 0,
            Pass::LightmappedGeneric {
                alpha: _,
                // TODO: Support the various kinds of envmap masks.
                env_map: Some(_),
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
    texture_path: VpkPath,
    tint: [u8; 3],
    mask: BatchEnvMapMask,
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum BatchEnvMapMask {
    None,
    Texture { path: VpkPath },
    BaseAlpha,
    BumpMapAlpha,
}

impl Batch {
    fn from_material_plane(material: &Vmt, plane: [FloatByBits; 3]) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                alpha_test_reference,
                base_alpha_env_map_mask,
                base_texture,
                env_map,
                env_map_mask,
                env_map_tint,
                normal_map_alpha_env_map_mask,
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
                        texture_path: env_map.path().clone(),
                        tint: [
                            ((env_map_tint[0] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                            ((env_map_tint[1] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                            ((env_map_tint[2] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                        ],
                        mask: match (
                            base_alpha_env_map_mask,
                            normal_map_alpha_env_map_mask,
                            env_map_mask,
                        ) {
                            (false, false, None) => BatchEnvMapMask::None,
                            (false, false, Some(env_map_mask)) => BatchEnvMapMask::Texture {
                                path: env_map_mask.path().clone(),
                            },
                            (true, false, None) => BatchEnvMapMask::BaseAlpha,
                            (false, true, None) =>  BatchEnvMapMask::BumpMapAlpha,
                            _ => panic!("bad env map mask combination: base_alpha_env_map_mask={}, normal_map_alpha_env_map_mask={}, env_map_mask={:?}",
                                base_alpha_env_map_mask,
                                normal_map_alpha_env_map_mask,
                                env_map_mask.as_ref().map(|vtf| vtf.path().as_canonical_path()),
                            ),
                        },
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
                    (cluster_lightmap.width, cluster_lightmap.height),
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
}

fn write_textures(
    dst_path: &Path,
    asset_loader: &AssetLoader,
    map_geometry: &MapGeometry,
) -> Result<HashMap<(VpkPath, Option<[FloatByBits; 3]>), u16>> {
    let mut planes_by_env_map: HashMap<VpkPath, HashSet<[FloatByBits; 3]>> = Default::default();
    for cluster_geometry in &map_geometry.clusters {
        for (_, batch) in cluster_geometry.display_lists_by_pass_batch.keys() {
            if let Some(env_map) = batch.env_map.as_ref() {
                planes_by_env_map
                    .entry(env_map.texture_path.clone())
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

    const GAMECUBE_MEMORY_BUDGET: usize = 20 * 1024 * 1024;
    for dimension_divisor in [1, 2, 4, 8, 16, 32] {
        let mut total_size = 0;
        for (type_, texture) in &textures {
            if texture.data().is_none() {
                bail!("no texture data for {}", texture.path());
            }
            let image_data = texture.data().unwrap();
            match type_ {
                TextureType::Image => {
                    let max_width = texture.width() / dimension_divisor;
                    let max_height = texture.height() / dimension_divisor;

                    // Take all mips that fit within the max_dimension.
                    let mut accepted_mip = false;
                    for face_mip in texture.iter_face_mips() {
                        assert_eq!(face_mip.face, 0);
                        if face_mip.texture.width() <= max_width
                            && face_mip.texture.height() <= max_height
                        {
                            match &image_data.mips[0][0] {
                                AnyTextureBuf::Dxt1(_) | AnyTextureBuf::Dxt5(_) => {
                                    total_size += GxTfCmpr::encoded_size(
                                        face_mip.texture.width(),
                                        face_mip.texture.height(),
                                    );
                                }
                                AnyTextureBuf::Bgr8(_) => {
                                    total_size += GxTfRgba8::encoded_size(
                                        face_mip.texture.width(),
                                        face_mip.texture.height(),
                                    );
                                }
                                texture => {
                                    bail!("unexpected texture format: {:?}", texture.format())
                                }
                            }
                            accepted_mip = true;
                        }
                    }

                    if !accepted_mip {
                        // TODO: Take the smallest available mip.

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
                    let width = 2 * texture.width() / dimension_divisor;
                    let height = 2 * texture.height() / dimension_divisor;

                    // Use RGBA8, making this 32 bpp.
                    // TODO: Use DXT1.
                    total_size += width.max(4) * height.max(4) * 4;
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
                    let max_width = texture.width() / dimension_divisor;
                    let max_height = texture.height() / dimension_divisor;

                    let start_offset = texture_data.stream_position()? as u32;
                    let mut mips_written = 0;
                    for face_mip in texture.iter_face_mips() {
                        assert_eq!(face_mip.face, 0);
                        if face_mip.texture.width() <= max_width
                            && face_mip.texture.height() <= max_height
                        {
                            match face_mip.texture {
                                AnyTextureBuf::Dxt1(texture) => {
                                    texture_data.write_all(
                                        TextureBuf::<GxTfCmpr>::encode(texture).data(),
                                    )?;
                                }
                                AnyTextureBuf::Dxt5(texture) => {
                                    texture_data.write_all(
                                        TextureBuf::<GxTfCmpr>::encode(texture).data(),
                                    )?;
                                }
                                AnyTextureBuf::Bgr8(texture) => {
                                    texture_data.write_all(
                                        TextureBuf::<GxTfRgba8>::encode(texture).data(),
                                    )?;
                                }
                                _ => unreachable!(),
                            }
                            mips_written += 1;
                        }
                    }
                    if mips_written == 0 {
                        // TODO: Take the smallest available mip.

                        bail!(
                            "unable to find a mipmap within max_size={}x{} for texture {}",
                            max_width,
                            max_height,
                            texture.path(),
                        );
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
                    texture_table.write_u8(match &image_data.mips[0][0] {
                        AnyTextureBuf::Dxt1(_) | AnyTextureBuf::Dxt5(_) => 0xe, // GX_TF_CMPR
                        AnyTextureBuf::Bgr8(_) => 0x6,                          // GX_TF_RGBA8
                        _ => unreachable!(),
                    })?;
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += (end_offset - start_offset) as usize;
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
                    match image_data.mips[0][0] {
                        AnyTextureBuf::Dxt1(_) => {
                            let encoded_faces = &image_data.mips[0];
                            assert!(encoded_faces.len() == 6 || encoded_faces.len() == 7);
                            for face_index in 0..6 {
                                faces.push(
                                    TextureBuf::<Rgba8>::encode_any(&encoded_faces[face_index])
                                        .into_data(),
                                );
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
                    texture_data.write_all(
                        TextureBuf::<GxTfRgba8>::encode(TextureBuf::<Rgba8>::new(
                            sphere_width,
                            sphere_height,
                            pixels,
                        ))
                        .data(),
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
                    total_size += (end_offset - start_offset) as usize;
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

    for cluster in &map_geometry.clusters {
        // Emit part of a ClusterGeometry struct to the table.
        table_file.write_u32::<BigEndian>(byte_code_file.index()? as u32)?;

        let mut prev_mode = None;
        let mut prev_base_texture_path = None;
        let mut prev_plane = None;
        let mut prev_env_map_path = None;
        let mut prev_env_map_tint = None;
        let mut prev_alpha = None;
        for ((pass, batch), display_list) in &cluster.display_lists_by_pass_batch {
            let mode = pass.as_mode();
            let plane = batch.env_map.as_ref().map(|env_map| env_map.plane);
            let env_map_path = batch.env_map.as_ref().map(|env_map| &env_map.texture_path);
            let env_map_tint = batch.env_map.as_ref().map(|env_map| env_map.tint);

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

            if prev_env_map_path != env_map_path {
                prev_env_map_path = env_map_path;

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

fn write_lightmaps(
    dst_path: &Path,
    bsp: Bsp,
    cluster_lightmaps: &HashMap<i16, ClusterLightmap>,
) -> Result<()> {
    let mut cluster_table_file =
        BufWriter::new(File::create(dst_path.join("lightmap_cluster_table.dat"))?);
    let mut patch_table_file = RecordWriter::new(
        BufWriter::new(File::create(dst_path.join("lightmap_patch_table.dat"))?),
        16,
    );
    let mut data_file = BufWriter::new(File::create(dst_path.join("lightmap_data.dat"))?);

    let cluster_end_index = cluster_lightmaps.keys().copied().max().unwrap();
    for cluster in 0..cluster_end_index {
        let lightmap = match cluster_lightmaps.get(&cluster) {
            Some(x) => x,
            None => continue,
        };

        let patch_table_start_index = patch_table_file.index()? as u32;
        for leaf in bsp.iter_worldspawn_leaves() {
            if leaf.cluster != cluster {
                continue;
            }

            let mut lightmap_patches_by_data_offset = HashMap::new();
            for face in bsp.iter_faces_from_leaf(leaf) {
                if face.light_ofs == -1 || face.tex_info == -1 {
                    continue;
                }
                let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;
                let metadata = lightmap.metadata_by_data_offset[&face.light_ofs];
                let tex_info = &bsp.tex_infos()[face.tex_info as usize];
                let style_count: u8 = face
                    .styles
                    .iter()
                    .map(|&x| if x != 255 { 1 } else { 0 })
                    .sum();
                assert!(style_count > 0);
                let bump_light = (tex_info.flags & 0x800) != 0;

                lightmap_patches_by_data_offset
                    .entry(face.light_ofs)
                    .or_insert_with(|| LightmapPatch {
                        width: u8::try_from(width).unwrap(),
                        height: u8::try_from(height).unwrap(),
                        style_count,
                        bump_light,
                        luxel_offset: metadata.luxel_offset,
                        is_flipped: metadata.is_flipped,
                    });
            }

            let mut data_offsets: Vec<_> =
                lightmap_patches_by_data_offset.keys().copied().collect();
            data_offsets.sort_unstable();
            for data_offset in data_offsets {
                let data_start_offset = data_file.stream_position()? as u32;

                let patch = &lightmap_patches_by_data_offset[&data_offset];
                assert_eq!(patch.luxel_offset[0] % 4, 0);
                assert_eq!(patch.luxel_offset[1] % 4, 0);
                let patch_size = 4 * patch.width as usize * patch.height as usize;
                let (oriented_width, oriented_height) = if patch.is_flipped {
                    (patch.height, patch.width)
                } else {
                    (patch.width, patch.height)
                };
                let blocks_wide = ((oriented_width as usize + 3) / 4).max(1);
                let blocks_high = ((oriented_height as usize + 3) / 4).max(1);

                let angle_count = if patch.bump_light { 4 } else { 1 };
                for style in 0..patch.style_count {
                    // Only export the first angle, which is the omnidirectional lightmap sample.
                    let angle = 0u8;

                    // Higher indexed styles come first. Angles are in increasing index order.
                    let patch_index =
                        (angle_count * (patch.style_count - style - 1) + angle) as usize;
                    let patch_base = data_offset as usize + patch_size * patch_index;

                    // Traverse blocks in texture format order.
                    for coarse_y in 0..blocks_high {
                        for coarse_x in 0..blocks_wide {
                            // Each block consists of individually packed AR and GB sub-blocks.
                            transcode_lightmap_patch_to_gamecube_rgba8_sub_block(
                                bsp,
                                patch,
                                patch_base,
                                4 * coarse_x,
                                4 * coarse_y,
                                |[r, _g, _b]| Ok(data_file.write_all(&[255, r])?),
                            )?;
                            transcode_lightmap_patch_to_gamecube_rgba8_sub_block(
                                bsp,
                                patch,
                                patch_base,
                                4 * coarse_x,
                                4 * coarse_y,
                                |[_r, g, b]| Ok(data_file.write_all(&[g, b])?),
                            )?;
                        }
                    }
                }
                let data_end_offset = data_file.stream_position()? as u32;

                patch_table_file.write_u8((patch.luxel_offset[0] / 4) as u8)?;
                patch_table_file.write_u8((patch.luxel_offset[1] / 4) as u8)?;
                patch_table_file.write_u8(blocks_wide as u8)?;
                patch_table_file.write_u8(blocks_high as u8)?;
                patch_table_file.write_u8(patch.style_count)?;
                patch_table_file.write_u8(0)?; // padding
                patch_table_file.write_u16::<BigEndian>(0)?; // padding
                patch_table_file.write_u32::<BigEndian>(data_start_offset)?;
                patch_table_file.write_u32::<BigEndian>(data_end_offset)?;
            }
        }
        let patch_table_end_index = patch_table_file.index()? as u32;

        cluster_table_file.write_u16::<BigEndian>(lightmap.width as u16)?;
        cluster_table_file.write_u16::<BigEndian>(lightmap.height as u16)?;
        cluster_table_file.write_u32::<BigEndian>(patch_table_start_index)?;
        cluster_table_file.write_u32::<BigEndian>(patch_table_end_index)?;
    }

    cluster_table_file.flush()?;
    patch_table_file.flush()?;
    data_file.flush()?;
    Ok(())
}

fn transcode_lightmap_patch_to_gamecube_rgba8_sub_block(
    bsp: Bsp,
    patch: &LightmapPatch,
    patch_base: usize,
    x0: usize,
    y0: usize,
    mut f: impl FnMut([u8; 3]) -> Result<()>,
) -> Result<()> {
    for fine_y in 0..4 {
        for fine_x in 0..4 {
            let dst_x = x0 + fine_x;
            let dst_y = y0 + fine_y;
            let (src_x, src_y) = if patch.is_flipped {
                (dst_y, dst_x)
            } else {
                (dst_x, dst_y)
            };
            if src_x < patch.width as usize && src_y < patch.height as usize {
                let src_offset =
                    patch_base + 4 * (patch.width as usize * src_y as usize + src_x as usize);
                let rgb = bsp.lighting().at_offset(src_offset, 1)[0].to_srgb8();
                f(rgb)?;
            } else {
                f([0; 3])?;
            }
        }
    }
    Ok(())
}
