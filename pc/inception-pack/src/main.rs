#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

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
use texture_format::{TextureBuf, TextureFormat};

#[cfg(test)]
use quickcheck::Arbitrary;

use crate::counter::Counter;
use crate::display_list::DisplayListBuilder;
use crate::packed_material::{
    PackedMaterial, PackedMaterialBaseAlpha, PackedMaterialEnvMapMask, TextureIdAllocator,
};
use crate::record_writer::RecordWriter;
use crate::texture_key::OwnedTextureKey;
use crate::write_big_endian::WriteBigEndian;

mod counter;
mod display_list;
mod packed_material;
mod record_writer;
mod texture_key;
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
        (@subcommand describe_texture =>
            (about: "prints texture metadata to stdout")
            (@arg NAME: ... "Texture name (example: tile/tilefloor013a)")
        )
    )
    .get_matches();

    let hl2_base = Path::new(matches.value_of("hl2_base").unwrap());
    match matches.subcommand() {
        ("pack_map", Some(matches)) => pack_map(hl2_base, matches)?,
        ("cat_material", Some(matches)) => cat_material(hl2_base, matches)?,
        ("describe_texture", Some(matches)) => describe_texture(hl2_base, matches)?,
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

fn describe_texture(hl2_base: &Path, matches: &ArgMatches) -> Result<()> {
    let material_loader = Rc::new(Vpk::new(hl2_base.join("hl2_misc"))?);
    let texture_loader = Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?);
    let asset_loader = AssetLoader::new(material_loader, texture_loader);
    for name in matches.values_of("NAME").unwrap() {
        let path = VpkPath::new_with_prefix_and_extension(name, "materials", "vtf");
        let vtf = asset_loader.get_texture(&path)?;
        println!("width: {}", vtf.width());
        println!("height: {}", vtf.height());
        println!("flags: 0x{:08x}", vtf.flags());
        println!("mips: {}", vtf.mips().len());
        println!("faces: {}", vtf.face_count());
        println!("format: {:?}", vtf.format());
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

    write_textures(dst_path, &asset_loader, &map_geometry)?;
    write_position_data(dst_path, &map_geometry.position_data)?;
    write_normal_data(dst_path, &map_geometry.normal_data)?;
    write_lightmap_coord_data(dst_path, &map_geometry.lightmap_coord_data)?;
    write_texture_coord_data(dst_path, &map_geometry.texture_coord_data)?;

    write_geometry(dst_path, &map_geometry)?;
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
    texture_keys: Vec<OwnedTextureKey>,
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
    display_lists_by_pass_material_params: BTreeMap<(Pass, PackedMaterial, ShaderParams), Vec<u8>>,
}

#[derive(Default)]
struct ClusterGeometryBuilder {
    display_lists_by_pass_material_params:
        BTreeMap<(Pass, PackedMaterial, ShaderParams), DisplayListBuilder>,
}

impl ClusterGeometryBuilder {
    pub fn display_list_builder(
        &mut self,
        pass: Pass,
        material: PackedMaterial,
        params: ShaderParams,
    ) -> &mut DisplayListBuilder {
        self.display_lists_by_pass_material_params
            .entry((pass, material, params))
            .or_insert_with(|| DisplayListBuilder::new(DisplayListBuilder::TRIANGLES))
    }

    pub fn build(self) -> ClusterGeometry {
        ClusterGeometry {
            display_lists_by_pass_material_params: self
                .display_lists_by_pass_material_params
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
        base_alpha: PackedMaterialBaseAlpha,
        env_map: Option<PassEnvMap>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PassAlpha {
    OpaqueOrAlphaTest,
    AlphaBlend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct PassEnvMap {
    mask: PackedMaterialEnvMapMask,
}

impl Pass {
    fn from_material(material: &Vmt, packed_material: &PackedMaterial) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                translucent,
                ..
            }) => Self::LightmappedGeneric {
                alpha: match (*alpha_test, *translucent) {
                    (_, false) => PassAlpha::OpaqueOrAlphaTest,
                    (false, true) => PassAlpha::AlphaBlend,
                    (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                },
                base_alpha: packed_material.base_alpha,
                env_map: packed_material
                    .env_map
                    .as_ref()
                    .map(|env_map| PassEnvMap { mask: env_map.mask }),
            },
            Shader::Unsupported => panic!(),
        }
    }

    fn as_mode(self) -> u8 {
        match self {
            // Disallowed combinations.
            Pass::LightmappedGeneric {
                alpha: _,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::BaseTextureAlpha,
                    }),
            } => unreachable!(
                "sampling base alpha for an env map mask, \
                but base alpha is packed in the aux texture"
            ),

            // # Opaque pass
            // ## Base texture alpha
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                env_map: None,
            } => 0,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::None,
                    }),
            } => 1,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::OpaqueOrAlphaTest,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::BaseTextureAlpha,
            //         }),
            // } => 2,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                    }),
            } => 3,

            // # Opaque pass (cont.)
            // ## Aux texture alpha
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map: None,
            } => 4,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::OpaqueOrAlphaTest,
            //     base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::None,
            //         }),
            // } => 5,
            // (disallowed) => 6,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::OpaqueOrAlphaTest,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                    }),
            } => 7,

            // # Blended pass
            // ## Base texture alpha
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map: None,
            // } => 8,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::None,
            //         }),
            // } => 9,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::BaseTextureAlpha,
            //         }),
            // } => 10,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::BaseTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
            //         }),
            // } => 11,

            // # Blended pass
            // ## Aux texture alpha
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map: None,
            } => 12,
            // Pass::LightmappedGeneric {
            //     alpha: PassAlpha::AlphaBlend,
            //     base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
            //     env_map:
            //         Some(PassEnvMap {
            //             mask: PackedMaterialEnvMapMask::None,
            //         }),
            // } => 13,
            // (disallowed) => 14,
            Pass::LightmappedGeneric {
                alpha: PassAlpha::AlphaBlend,
                base_alpha: PackedMaterialBaseAlpha::AuxTextureAlpha,
                env_map:
                    Some(PassEnvMap {
                        mask: PackedMaterialEnvMapMask::AuxTextureIntensity,
                    }),
            } => 15,
            _ => panic!("unexpected pass: {:?}", self),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ShaderParams {
    // Order matters here! `plane` is the first field to minimize plane changes in the display byte
    // code.
    plane: Plane,
    env_map_tint: [u8; 3],
    alpha: ShaderParamsAlpha,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShaderParamsAlpha {
    Opaque,
    AlphaTest { threshold: u8 },
    AlphaBlend,
}

impl ShaderParams {
    fn from_material_plane(material: &Vmt, plane: Plane) -> Self {
        match material.shader() {
            Shader::LightmappedGeneric(LightmappedGeneric {
                alpha_test,
                alpha_test_reference,
                env_map_tint,
                translucent,
                ..
            }) => {
                let env_map_tint = env_map_tint.unwrap_or(vec3(1.0, 1.0, 1.0));
                Self {
                    plane,
                    env_map_tint: [
                        ((env_map_tint[0] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                        ((env_map_tint[1] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                        ((env_map_tint[2] * 255.0).clamp(0.0, 255.0) + 0.5) as u8,
                    ],
                    alpha: match (*alpha_test, *translucent) {
                        (false, false) => ShaderParamsAlpha::Opaque,
                        (false, true) => ShaderParamsAlpha::AlphaBlend,
                        (true, false) => ShaderParamsAlpha::AlphaTest {
                            threshold: ((alpha_test_reference * 255.0).clamp(0.0, 255.0) + 0.5)
                                as u8,
                        },
                        (true, true) => panic!("material is both alpha-tested and alpha-blended"),
                    },
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
    // Pre-pass: Collect the set of unique planes for each material.
    let mut material_planes: HashMap<VpkPath, HashSet<Plane>> = HashMap::new();
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

            let tex_info = &bsp.tex_infos()[face.tex_info as usize];
            if tex_info.tex_data == -1 {
                // Not textured.
                // TODO: Determine whether any such faces need to be drawn.
                continue;
            }

            let tex_data = &bsp.tex_datas()[tex_info.tex_data as usize];
            let material_path = VpkPath::new_with_prefix_and_extension(
                bsp.tex_data_strings()
                    .get(tex_data.name_string_table_id as usize),
                "materials",
                "vmt",
            );
            let plane = Plane::from(bsp.planes()[face.plane_num as usize].normal);
            material_planes
                .entry(material_path.clone())
                .or_default()
                .insert(plane);
        }
    }

    let mut positions = AttributeBuilder::new();
    let mut normals = AttributeBuilder::new();
    let mut lightmap_coords = AttributeBuilder::new();
    let mut texture_coords = AttributeBuilder::new();
    let mut clusters: Vec<ClusterGeometryBuilder> = Vec::new();
    let mut ids = TextureIdAllocator::new();
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
            let plane = Plane::from(bsp.planes()[face.plane_num as usize].normal);
            let material = asset_loader.get_material(&material_path)?;
            let base_texture_size = match material.shader() {
                Shader::LightmappedGeneric(LightmappedGeneric {
                    base_texture_path, ..
                }) => {
                    let base_texture = asset_loader.get_texture(base_texture_path)?;
                    [base_texture.width() as f32, base_texture.height() as f32]
                }
                _ => continue,
            };
            let packed_material = PackedMaterial::from_material_and_all_planes(
                asset_loader,
                &mut ids,
                &material,
                &material_planes[&material_path],
            )?;
            let mut polygon_builder = PolygonBuilder::new(cluster_builder.display_list_builder(
                Pass::from_material(&material, &packed_material),
                packed_material,
                ShaderParams::from_material_plane(&material, plane),
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
        texture_keys: ids.into_keys(),
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

#[cfg(test)]
impl Arbitrary for FloatByBits {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self(f32::arbitrary(g))
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Plane {
    x: FloatByBits,
    y: FloatByBits,
    z: FloatByBits,
}

impl Plane {
    pub fn to_vec3(&self) -> Vec3 {
        vec3(self.x.0, self.y.0, self.z.0)
    }
}

#[cfg(test)]
impl Arbitrary for Plane {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            x: FloatByBits::arbitrary(g),
            y: FloatByBits::arbitrary(g),
            z: FloatByBits::arbitrary(g),
        }
    }
}

impl From<[f32; 3]> for Plane {
    fn from(xyz: [f32; 3]) -> Self {
        let [x, y, z] = hashable_float(&xyz);
        Self { x, y, z }
    }
}

fn write_textures(
    dst_path: &Path,
    asset_loader: &AssetLoader,
    map_geometry: &MapGeometry,
) -> Result<()> {
    fn get_dst_format(src_format: TextureFormat) -> Result<TextureFormat> {
        Ok(match src_format {
            TextureFormat::Dxt1 | TextureFormat::Dxt5 => TextureFormat::GxTfCmpr,
            TextureFormat::Bgr8 => TextureFormat::GxTfRgba8,
            format => {
                bail!("unexpected texture format: {:?}", format)
            }
        })
    }

    const GAMECUBE_MEMORY_BUDGET: usize = 8 * 1024 * 1024;
    for dimension_divisor in [1, 2, 4, 8, 16, 32] {
        let mut total_size = 0;
        for key in &map_geometry.texture_keys {
            match key {
                OwnedTextureKey::EncodeAsIs { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    let max_width = texture.width() / dimension_divisor;
                    let max_height = texture.height() / dimension_divisor;

                    // Take all mips that fit within the max_dimension.
                    let mut accepted_mip = false;
                    for face_mip in texture.iter_face_mips() {
                        assert_eq!(face_mip.face, 0);
                        if face_mip.texture.width() <= max_width
                            && face_mip.texture.height() <= max_height
                        {
                            let dst_format = get_dst_format(texture.format())?;
                            total_size += dst_format
                                .metrics()
                                .encoded_size(face_mip.texture.width(), face_mip.texture.height());
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

                OwnedTextureKey::Intensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    let max_width = texture.width() / dimension_divisor;
                    let max_height = texture.height() / dimension_divisor;

                    // Take all mips that fit within the max_dimension.
                    let mut accepted_mip = false;
                    for face_mip in texture.iter_face_mips() {
                        assert_eq!(face_mip.face, 0);
                        if face_mip.texture.width() <= max_width
                            && face_mip.texture.height() <= max_height
                        {
                            total_size += TextureFormat::GxTfI8
                                .metrics()
                                .encoded_size(face_mip.texture.width(), face_mip.texture.height());
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

                OwnedTextureKey::AlphaToIntensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    let max_width = texture.width() / dimension_divisor;
                    let max_height = texture.height() / dimension_divisor;

                    // Take all mips that fit within the max_dimension.
                    let mut accepted_mip = false;
                    for face_mip in texture.iter_face_mips() {
                        assert_eq!(face_mip.face, 0);
                        if face_mip.texture.width() <= max_width
                            && face_mip.texture.height() <= max_height
                        {
                            total_size += TextureFormat::GxTfI8
                                .metrics()
                                .encoded_size(face_mip.texture.width(), face_mip.texture.height());
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

                OwnedTextureKey::ComposeIntensityAlpha {
                    intensity_texture_path,
                    alpha_texture_path,
                } => {
                    let intensity_texture = asset_loader.get_texture(intensity_texture_path)?;
                    let alpha_texture = asset_loader.get_texture(alpha_texture_path)?;
                    assert_eq!(intensity_texture.width(), alpha_texture.width());
                    assert_eq!(intensity_texture.height(), alpha_texture.height());
                    assert_eq!(intensity_texture.mips().len(), alpha_texture.mips().len());

                    let max_width = intensity_texture.width() / dimension_divisor;
                    let max_height = intensity_texture.height() / dimension_divisor;

                    // Take all mips that fit within the max_dimension.
                    let mut accepted_mip = false;
                    for face_mip in intensity_texture.iter_face_mips() {
                        assert_eq!(face_mip.face, 0);
                        if face_mip.texture.width() <= max_width
                            && face_mip.texture.height() <= max_height
                        {
                            total_size += TextureFormat::GxTfIa8
                                .metrics()
                                .encoded_size(face_mip.texture.width(), face_mip.texture.height());
                            accepted_mip = true;
                        }
                    }

                    if !accepted_mip {
                        // TODO: Take the smallest available mip.

                        bail!(
                            "unable to find a mipmap within max_size={}x{} for texture {}",
                            max_width,
                            max_height,
                            intensity_texture.path(),
                        );
                    }
                }

                OwnedTextureKey::BakeOrientedEnvmap { texture_path, .. } => {
                    // Emit a sphere map at double the cube face dimension.
                    let texture = asset_loader.get_texture(texture_path)?;
                    let width = 2 * texture.width() / dimension_divisor;
                    let height = 2 * texture.height() / dimension_divisor;
                    // TODO: Use DXT1.
                    total_size += TextureFormat::GxTfRgba8
                        .metrics()
                        .encoded_size(width, height);
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
        for key in &map_geometry.texture_keys {
            match key {
                OwnedTextureKey::EncodeAsIs { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.face_count(), 1);
                    let dst_format = get_dst_format(texture.format())?;

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
                            texture_data.write_all(
                                TextureBuf::transcode(face_mip.texture.as_slice(), dst_format)
                                    .data(),
                            )?;
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
                    texture_table.write_u8(gx_texture_format(dst_format))?;
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += (end_offset - start_offset) as usize;
                }

                OwnedTextureKey::Intensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.face_count(), 1);
                    let dst_format = TextureFormat::GxTfI8;

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
                            texture_data.write_all(
                                TextureBuf::transcode(face_mip.texture.as_slice(), dst_format)
                                    .data(),
                            )?;
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
                    texture_table.write_u8(gx_texture_format(dst_format))?;
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += (end_offset - start_offset) as usize;
                }

                OwnedTextureKey::AlphaToIntensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.face_count(), 1);
                    let dst_format = TextureFormat::GxTfI8;

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
                            // Broadcast alpha to all channels.
                            let mut texel_data = TextureBuf::transcode(
                                face_mip.texture.as_slice(),
                                TextureFormat::Rgba8,
                            )
                            .into_data();
                            for texel_index in
                                0..face_mip.texture.width() * face_mip.texture.height()
                            {
                                let offset = 4 * texel_index;
                                texel_data[offset] = texel_data[offset + 3];
                                texel_data[offset + 1] = texel_data[offset + 3];
                                texel_data[offset + 2] = texel_data[offset + 3];
                            }

                            texture_data.write_all(
                                TextureBuf::transcode(
                                    TextureBuf::new(
                                        TextureFormat::Rgba8,
                                        face_mip.texture.width(),
                                        face_mip.texture.height(),
                                        texel_data,
                                    )
                                    .as_slice(),
                                    dst_format,
                                )
                                .data(),
                            )?;
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
                    texture_table.write_u8(gx_texture_format(dst_format))?;
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += (end_offset - start_offset) as usize;
                }

                OwnedTextureKey::ComposeIntensityAlpha {
                    intensity_texture_path,
                    alpha_texture_path,
                } => {
                    let intensity_texture = asset_loader.get_texture(intensity_texture_path)?;
                    let alpha_texture = asset_loader.get_texture(alpha_texture_path)?;
                    assert_eq!(intensity_texture.width(), alpha_texture.width());
                    assert_eq!(intensity_texture.height(), alpha_texture.height());
                    assert_eq!(intensity_texture.flags() & 0xc, alpha_texture.flags() & 0xc);
                    assert_eq!(intensity_texture.mips().len(), alpha_texture.mips().len());
                    assert_eq!(intensity_texture.face_count(), 1);
                    assert_eq!(alpha_texture.face_count(), 1);
                    let dst_format = TextureFormat::GxTfIa8;

                    // Take all mips that fit within the max_dimension.
                    let max_width = intensity_texture.width() / dimension_divisor;
                    let max_height = intensity_texture.height() / dimension_divisor;

                    let start_offset = texture_data.stream_position()? as u32;
                    let mut mips_written = 0;
                    for (mip_level, (intensity_faces, alpha_faces)) in intensity_texture
                        .mips()
                        .iter()
                        .zip(alpha_texture.mips().iter())
                        .enumerate()
                    {
                        let intensity_mip = &intensity_faces[0];
                        let alpha_mip = &alpha_faces[0];

                        let width = (intensity_texture.width() >> mip_level).max(1);
                        let height = (intensity_texture.height() >> mip_level).max(1);

                        if width <= max_width && height <= max_height {
                            // Combine the intensity and alpha textures by channel into a new
                            // texture.
                            let intensity_data = TextureBuf::transcode(
                                intensity_mip.as_slice(),
                                TextureFormat::Rgba8,
                            )
                            .into_data();
                            let alpha_data =
                                TextureBuf::transcode(alpha_mip.as_slice(), TextureFormat::Rgba8)
                                    .into_data();
                            let mut texels = Vec::with_capacity(4 * width * height);
                            for texel_index in 0..width * height {
                                let offset = 4 * texel_index;
                                texels.extend_from_slice(&[
                                    intensity_data[offset],
                                    intensity_data[offset + 1],
                                    intensity_data[offset + 2],
                                    alpha_data[offset + 3],
                                ]);
                            }

                            texture_data.write_all(
                                TextureBuf::transcode(
                                    TextureBuf::new(TextureFormat::Rgba8, width, height, texels)
                                        .as_slice(),
                                    dst_format,
                                )
                                .data(),
                            )?;
                            mips_written += 1;
                        }
                    }
                    if mips_written == 0 {
                        // TODO: Take the smallest available mip.

                        bail!(
                            "unable to find a mipmap within max_size={}x{} for textures {} and {}",
                            max_width,
                            max_height,
                            intensity_texture_path,
                            alpha_texture_path,
                        );
                    }
                    // Pad to a 32 byte boundary.
                    while (texture_data.stream_position()? & 31) != 0 {
                        texture_data.write_u8(0)?;
                    }
                    let end_offset = texture_data.stream_position()? as u32;

                    // Write a texture table entry.
                    texture_table.write_u16::<BigEndian>(
                        (intensity_texture.width() / dimension_divisor) as u16,
                    )?;
                    texture_table.write_u16::<BigEndian>(
                        (intensity_texture.height() / dimension_divisor) as u16,
                    )?;
                    texture_table.write_u8(mips_written)?;
                    texture_table.write_u8(
                        if (intensity_texture.flags() & 0x4) != 0 {
                            0x01
                        } else {
                            0
                        } | if (intensity_texture.flags() & 0x8) != 0 {
                            0x02
                        } else {
                            0
                        },
                    )?;
                    texture_table.write_u8(gx_texture_format(dst_format))?;
                    texture_table.write_u8(0)?;
                    texture_table.write_u32::<BigEndian>(start_offset)?;
                    texture_table.write_u32::<BigEndian>(end_offset)?;
                    total_size += (end_offset - start_offset) as usize;
                }

                OwnedTextureKey::BakeOrientedEnvmap {
                    texture_path,
                    plane,
                } => {
                    // Emit a sphere map at double the cube face dimension.
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.width(), texture.height());
                    let cube_size = texture.width() as usize;
                    let sphere_width = 2 * cube_size / dimension_divisor as usize;
                    let sphere_height = 2 * cube_size / dimension_divisor as usize;
                    let mut pixels = Vec::with_capacity(sphere_width * sphere_height);

                    // Decode the six cube map faces to flat RGBA8 buffers.
                    let mut faces = Vec::new();
                    {
                        let encoded_faces = &texture.mips()[0];
                        assert!(encoded_faces.len() == 6 || encoded_faces.len() == 7);
                        for face_index in 0..6 {
                            faces.push(
                                TextureBuf::transcode(
                                    encoded_faces[face_index].as_slice(),
                                    TextureFormat::Rgba8,
                                )
                                .into_data(),
                            );
                        }
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
                    let normal = plane.to_vec3();
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
                        TextureBuf::transcode(
                            TextureBuf::new(
                                TextureFormat::Rgba8,
                                sphere_width,
                                sphere_height,
                                pixels,
                            )
                            .as_slice(),
                            // TODO: Use DXT1.
                            TextureFormat::GxTfRgba8,
                        )
                        .data(),
                    )?;
                    let end_offset = texture_data.stream_position()? as u32;

                    // Write a texture table entry.
                    texture_table.write_u16::<BigEndian>(sphere_width as u16)?;
                    texture_table.write_u16::<BigEndian>(sphere_height as u16)?;
                    texture_table.write_u8(1)?; // mip count
                    texture_table.write_u8(0x3)?; // flags: CLAMP_S | CLAMP_T
                    texture_table.write_u8(gx_texture_format(TextureFormat::GxTfRgba8))?;
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

        return Ok(());
    }
    bail!("Unable to fit textures within the memory budget.");
}

fn gx_texture_format(format: TextureFormat) -> u8 {
    match format {
        TextureFormat::GxTfI8 => 0x1,
        TextureFormat::GxTfIa8 => 0x3,
        TextureFormat::GxTfRgba8 => 0x6,
        TextureFormat::GxTfCmpr => 0xe,
        _ => unreachable!(),
    }
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

    use crate::ShaderParamsAlpha;

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

    pub fn set_alpha(alpha: ShaderParamsAlpha) -> [u32; 1] {
        let test: u32 = match alpha {
            ShaderParamsAlpha::AlphaTest { .. } => 1,
            _ => 0,
        };
        let threshold: u32 = match alpha {
            ShaderParamsAlpha::AlphaTest { threshold } => threshold as u32,
            _ => 0,
        };
        let blend: u32 = match alpha {
            ShaderParamsAlpha::AlphaBlend => 1,
            _ => 0,
        };
        [0x05000000 | (test << 16) | (threshold << 8) | blend]
    }

    pub fn set_aux_texture(aux_texture_index: u16) -> [u32; 1] {
        [0x06000000 | aux_texture_index as u32]
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

fn write_geometry(dst_path: &Path, map_geometry: &MapGeometry) -> Result<()> {
    // TODO: Transpose this table for potential cache friendliness.

    // struct ClusterGeometry {
    //     pass_index_ranges: [[u32; 2]; 16],
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
        let table_start_offset = table_file.stream_position()?;

        for mode in 0..16 {
            // Write ClusterGeometry.pass_index_ranges[mode][0].
            table_file.write_u32::<BigEndian>(byte_code_file.index()? as u32)?;

            let mut prev_base_id = None;
            let mut prev_aux_id = None;
            let mut prev_plane = None;
            let mut prev_env_map_id = None;
            let mut prev_env_map_tint = None;
            let mut prev_alpha = None;
            for ((pass, material, params), display_list) in
                &cluster.display_lists_by_pass_material_params
            {
                if pass.as_mode() == mode {
                    if prev_base_id != Some(material.base_id) {
                        prev_base_id = Some(material.base_id);

                        byte_code::set_base_texture(material.base_id)
                            .write_big_endian_to(&mut *byte_code_file)?;
                    }

                    if let Some(aux_id) = material.aux_id {
                        if prev_aux_id != Some(material.aux_id) {
                            prev_aux_id = Some(material.aux_id);

                            byte_code::set_aux_texture(aux_id)
                                .write_big_endian_to(&mut *byte_code_file)?;
                        }
                    }

                    if let Some(env_map) = material.env_map.as_ref() {
                        if prev_plane != Some(params.plane) {
                            prev_plane = Some(params.plane);

                            let normal = params.plane.to_vec3();
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
                            let texture_matrix: Mat3x4 =
                                local_to_texture * local_reflect * world_to_local;

                            byte_code::set_plane(texture_matrix)
                                .write_big_endian_to(&mut *byte_code_file)?;
                        }

                        let env_map_id = env_map.ids_by_plane[&params.plane];
                        if prev_env_map_id != Some(env_map_id) {
                            prev_env_map_id = Some(env_map_id);

                            byte_code::set_env_map_texture(env_map_id)
                                .write_big_endian_to(&mut *byte_code_file)?;
                        }

                        if prev_env_map_tint != Some(params.env_map_tint) {
                            prev_env_map_tint = Some(params.env_map_tint);

                            byte_code::set_env_map_tint(params.env_map_tint)
                                .write_big_endian_to(&mut *byte_code_file)?;
                        }
                    }

                    if prev_alpha != Some(params.alpha) {
                        prev_alpha = Some(params.alpha);

                        byte_code::set_alpha(params.alpha)
                            .write_big_endian_to(&mut *byte_code_file)?;
                    }

                    let display_list_start_offset = display_lists_file.stream_position()? as u32;
                    assert_eq!(display_list_start_offset & 31, 0);
                    display_lists_file.write_all(display_list)?;
                    let display_list_end_offset = display_lists_file.stream_position()? as u32;
                    assert_eq!(display_list_end_offset & 31, 0);

                    byte_code::draw(display_list_start_offset, display_list_end_offset)
                        .write_big_endian_to(&mut *byte_code_file)?;
                }
            }

            // Write ClusterGeometry.pass_index_ranges[mode][1].
            table_file.write_u32::<BigEndian>(byte_code_file.index()? as u32)?;
        }

        let table_end_offset = table_file.stream_position()?;
        assert_eq!(table_end_offset - table_start_offset, 128);
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
