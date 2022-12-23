use std::collections::{BTreeMap, HashMap};
use std::fs::{create_dir_all, File};
use std::hash::Hash;
use std::io::Write;

use std::path::Path;
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use byteorder::{BigEndian, WriteBytesExt};
use gx::display_list::{DisplayList, GxPrimitive};
use inception_render_common::bytecode::BytecodeOp;
use inception_render_common::map_data::{
    BspLeaf, BspNode, ClusterGeometryReferencesEntry, ClusterGeometryTableEntry,
    ClusterLightmapTableEntry, CommonLightmapTableEntry, DisplacementLightmapTableEntry,
    DisplacementReferencesEntry, DisplacementTableEntry, LightmapPatchTableEntry, OwnedMapData,
    TextureTableEntry, WriteTo,
};
use memmap::Mmap;
use nalgebra_glm::{lerp, vec2, vec3, Mat2x3, Vec2, Vec3};
use num_traits::PrimInt;
use ordered_float::NotNan;
use source_reader::asset::vmt::{
    LightmappedGeneric, Shader, Sky, UnlitGeneric, WorldVertexTransition,
};
use source_reader::asset::vtf::{Vtf, VtfFaceMip};
use source_reader::asset::AssetLoader;
use source_reader::bsp::{Bsp, DispInfo, Face};
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::FallbackFileLoader;
use source_reader::geometry::{convert_vertex, Vertex};
use source_reader::lightmap::{build_lightmaps, Lightmap, LightmapMetadata, LightmapPatch};
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;
use texture_format::{TextureBuf, TextureFormat};

use crate::counter::Counter;
use crate::draw_builder::DrawBuilder;
use crate::gx_helpers::DisplayListExt;
use crate::legacy_pass_params::{DisplacementPass, Pass, ShaderParams, ShaderParamsAlpha};
use crate::packed_material::PackedMaterial;
use crate::texture_key::{OwnedTextureKey, TextureIdAllocator};
use crate::write_big_endian::WriteBigEndian;
use crate::{hashable_float, FloatByBits};

pub fn pack_map(hl2_base: &Path, dst: &Path, map_name_or_path: &str) -> Result<()> {
    let map_path = if map_name_or_path.ends_with(".bsp") {
        map_name_or_path.into()
    } else {
        let mut path = hl2_base.join("maps");
        path.push(format!("{}.bsp", map_name_or_path));
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

    let (cluster_lightmaps, displacement_lightmaps) = build_lightmaps(bsp)?;
    let map_geometry = process_geometry(
        bsp,
        &cluster_lightmaps,
        &displacement_lightmaps,
        &asset_loader,
    )?;

    let (texture_table, texture_data) = pack_textures(&asset_loader, &map_geometry)?;
    let (
        cluster_geometry_table,
        cluster_geometry_byte_code,
        cluster_geometry_display_lists,
        cluster_geometry_references,
    ) = pack_brush_geometry(&map_geometry, &texture_table);
    let bsp_nodes = pack_bsp_nodes(bsp);
    let bsp_leaves = pack_bsp_leaves(bsp);
    let visibility = pack_visibility(bsp);
    let (lightmap_cluster_table, lightmap_displacement_table, lightmap_patch_table, lightmap_data) =
        pack_lightmaps(bsp, &cluster_lightmaps, &displacement_lightmaps);
    let (
        displacement_table,
        displacement_byte_code,
        displacement_display_lists,
        displacement_references,
    ) = pack_displacement_geometry(&map_geometry, &texture_table);

    let dst_path = dst.join("maps");
    create_dir_all(&dst_path)?;

    let dst_file_name = format!("{}.dat", map_path.file_stem().unwrap().to_str().unwrap());
    let mut file = File::create(&dst_path.join(dst_file_name))?;
    OwnedMapData {
        position_data: map_geometry.position_data,
        normal_data: map_geometry.normal_data,
        texture_coord_data: map_geometry.texture_coord_data,
        cluster_geometry_table,
        cluster_geometry_byte_code,
        cluster_geometry_display_lists,
        cluster_geometry_references,
        bsp_nodes,
        bsp_leaves,
        visibility,
        texture_table,
        texture_data,
        lightmap_cluster_table,
        lightmap_displacement_table,
        lightmap_patch_table,
        lightmap_data,
        displacement_position_data: map_geometry.displacement_position_data,
        displacement_vertex_color_data: map_geometry.displacement_vertex_color_data,
        displacement_texture_coordinate_data: map_geometry.displacement_texture_coordinate_data,
        displacement_table,
        displacement_byte_code,
        displacement_display_lists,
        displacement_references,
    }
    .write_to(&mut file)?;
    file.flush()?;

    Ok(())
}

struct MapGeometry {
    position_data: Vec<u8>,
    normal_data: Vec<u8>,
    texture_coord_data: Vec<u8>,
    clusters: Vec<ClusterGeometry>,
    displacement_position_data: Vec<u8>,
    displacement_vertex_color_data: Vec<u8>,
    displacement_texture_coordinate_data: Vec<u8>,
    displacement_display_lists_by_pass_face_material:
        BTreeMap<(DisplacementPass, u16, PackedMaterial), DisplayList>,
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
    draw_builder: &'a mut DrawBuilder,
}

impl<'a, Vertex: Copy + WriteBigEndian> PolygonBuilder<'a, Vertex> {
    pub fn new(draw_builder: &'a mut DrawBuilder) -> Self {
        Self {
            first_vertex: None,
            prev_vertex: None,
            draw_builder,
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
            self.draw_builder.emit_vertices(3, &data);
        }
        self.prev_vertex = Some(vertex);
        Ok(())
    }
}

struct ClusterGeometry {
    display_lists_by_pass_material_params:
        BTreeMap<(Pass, PackedMaterial, ShaderParams), DisplayList>,
}

#[derive(Default)]
struct ClusterGeometryBuilder {
    draw_builders_by_pass_material_params:
        BTreeMap<(Pass, PackedMaterial, ShaderParams), DrawBuilder>,
}

impl ClusterGeometryBuilder {
    pub fn draw_builder(
        &mut self,
        pass: Pass,
        material: PackedMaterial,
        params: ShaderParams,
    ) -> &mut DrawBuilder {
        self.draw_builders_by_pass_material_params
            .entry((pass, material, params))
            .or_insert_with(|| DrawBuilder::new(GxPrimitive::Triangles, 0))
    }

    pub fn build(self) -> ClusterGeometry {
        ClusterGeometry {
            display_lists_by_pass_material_params: self
                .draw_builders_by_pass_material_params
                .into_iter()
                .map(|(key, draw_builder)| (key, draw_builder.build()))
                .filter(|(_, display_list)| !display_list.commands.is_empty())
                .collect(),
        }
    }
}

fn process_geometry(
    bsp: Bsp,
    cluster_lightmaps: &HashMap<i16, Lightmap>,
    displacement_lightmaps: &HashMap<u16, Lightmap>,
    asset_loader: &AssetLoader,
) -> Result<MapGeometry> {
    let mut ids = TextureIdAllocator::new();
    // The first five texture IDs are reserved for the 2D skybox.
    allocate_skybox_textures(bsp, asset_loader, &mut ids)?;

    let mut positions = AttributeBuilder::new();
    let mut normals = AttributeBuilder::new();
    let mut texture_coords = AttributeBuilder::new();
    let mut clusters: Vec<ClusterGeometryBuilder> = Vec::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        let cluster = leaf.cluster();
        if cluster == -1 {
            // Leaf is not potentially visible from anywhere.
            continue;
        }
        if clusters.len() < (cluster as usize + 1) {
            clusters.resize_with(cluster as usize + 1, Default::default);
        }
        let cluster_builder = &mut clusters[cluster as usize];
        let lightmap = cluster_lightmaps.get(&cluster);

        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.tex_info != -1 {
                process_textured_brush_face(
                    bsp,
                    asset_loader,
                    &mut ids,
                    &mut positions,
                    &mut normals,
                    &mut texture_coords,
                    cluster_builder,
                    lightmap,
                    face,
                )?;
            }
        }
    }

    let mut displacement_positions = AttributeBuilder::new();
    let mut displacement_vertex_colors = AttributeBuilder::new();
    let mut displacement_texture_coordinates = AttributeBuilder::new();
    let mut displacement_draw_builders_by_pass_face_material = BTreeMap::new();
    for disp_info in bsp.disp_infos() {
        let lightmap = displacement_lightmaps.get(&disp_info.map_face);
        process_displacement(
            bsp,
            asset_loader,
            &mut ids,
            &mut displacement_positions,
            &mut displacement_vertex_colors,
            &mut displacement_texture_coordinates,
            &mut displacement_draw_builders_by_pass_face_material,
            lightmap,
            disp_info,
        )?;
    }
    let displacement_display_lists_by_pass_face_material =
        displacement_draw_builders_by_pass_face_material
            .into_iter()
            .map(|(key, builder)| (key, builder.build()))
            .collect();

    Ok(MapGeometry {
        position_data: positions.build(),
        normal_data: normals.build(),
        texture_coord_data: texture_coords.build(),
        clusters: clusters
            .into_iter()
            .map(ClusterGeometryBuilder::build)
            .collect(),
        displacement_position_data: displacement_positions.build(),
        displacement_vertex_color_data: displacement_vertex_colors.build(),
        displacement_texture_coordinate_data: displacement_texture_coordinates.build(),
        displacement_display_lists_by_pass_face_material,
        texture_keys: ids.into_keys(),
    })
}

fn allocate_skybox_textures(
    bsp: Bsp,
    asset_loader: &AssetLoader,
    ids: &mut TextureIdAllocator,
) -> Result<()> {
    let entities = bsp.entities();
    let worldspawn = &entities[0];

    for face in ["rt", "lf", "bk", "ft", "up"] {
        let material = asset_loader.get_material(&VpkPath::new_with_prefix_and_extension(
            &format!("{}{}", &worldspawn["skyname"], face),
            "materials/skybox",
            "vmt",
        ))?;
        let texture_path: &VpkPath = match material.shader() {
            Shader::UnlitGeneric(UnlitGeneric {
                base_texture_path, ..
            }) => base_texture_path,

            Shader::Sky(Sky { base_texture_path }) => base_texture_path,

            shader => panic!(
                "Unexpected skybox shader {:?} in {}",
                shader.name(),
                material.path(),
            ),
        };
        ids.get_force_unique(&OwnedTextureKey::EncodeAsIs {
            texture_path: texture_path.to_owned(),
        });
    }

    Ok(())
}

fn process_displacement(
    bsp: Bsp,
    asset_loader: &AssetLoader,
    ids: &mut TextureIdAllocator,
    positions: &mut AttributeBuilder<[FloatByBits; 3], u16>,
    vertex_colors: &mut AttributeBuilder<[u8; 3], u16>,
    texture_coordinates: &mut AttributeBuilder<[u16; 2], u16>,
    draw_builders_by_pass_face_material: &mut BTreeMap<
        (DisplacementPass, u16, PackedMaterial),
        DrawBuilder,
    >,
    lightmap: Option<&Lightmap>,
    disp_info: &DispInfo,
) -> Result<()> {
    let verts_per_side = (1 << disp_info.power) + 1;
    let vert_count = verts_per_side * verts_per_side;
    let vert_index = disp_info.disp_vert_start as usize;
    let verts = &bsp.disp_verts()[vert_index..vert_index + vert_count];

    let quads_per_side = 1 << disp_info.power;
    let tri_count = 2 * quads_per_side * quads_per_side;
    let tri_index = disp_info.disp_tri_start as usize;
    let _tris = &bsp.disp_tris()[tri_index..tri_index + tri_count];

    let face = &bsp.faces()[disp_info.map_face as usize];
    assert_eq!(face.num_edges, 4);
    assert_ne!(face.tex_info, -1);
    let lightmap_metadata =
        lightmap.map(|lightmap| lightmap.metadata_by_data_offset[&face.light_ofs]);
    let mut corners = [Vec3::zeros(); 4];
    let mut closest_distance = f32::INFINITY;
    let mut closest_corner = 0;
    for i in 0..4 {
        let mut edge_index = bsp.surf_edges()[face.first_edge as usize + i];
        let was_negative = edge_index < 0;
        if was_negative {
            edge_index *= -1;
        }
        let edge = &bsp.edges()[edge_index as usize];
        corners[i] = bsp.vertices()[edge.v[was_negative as usize] as usize];

        let distance = (disp_info.start_position_vec() - corners[i]).magnitude();
        if closest_distance > distance {
            closest_distance = distance;
            closest_corner = i;
        }
    }

    let tex_info = &bsp.tex_infos()[face.tex_info as usize];
    assert_ne!(tex_info.tex_data, -1);
    let tex_data = &bsp.tex_datas()[tex_info.tex_data as usize];
    let material_path = VpkPath::new_with_prefix_and_extension(
        bsp.tex_data_strings()
            .get(tex_data.name_string_table_id as usize),
        "materials",
        "vmt",
    );
    let material = asset_loader.get_material(&material_path)?;
    let packed_material =
        PackedMaterial::from_material(asset_loader, ids, &material, true)?.unwrap();
    let (pass, base_texture, texture_transform1, texture_transform2) = match material.shader() {
        Shader::LightmappedGeneric(LightmappedGeneric {
            base_texture_path, ..
        }) => (
            DisplacementPass::LightmappedGeneric,
            asset_loader.get_texture(base_texture_path)?,
            Mat2x3::identity(),
            Mat2x3::identity(),
        ),
        Shader::WorldVertexTransition(WorldVertexTransition {
            base_texture_path,
            base_texture_transform,
            base_texture_transform2,
            ..
        }) => (
            DisplacementPass::WorldVertexTransition,
            asset_loader.get_texture(base_texture_path)?,
            *base_texture_transform,
            *base_texture_transform2,
        ),
        shader => panic!("Unexpected shader: {:?}", shader.name()),
    };
    let display_list_builder = draw_builders_by_pass_face_material
        .entry((pass, disp_info.map_face, packed_material))
        .or_insert_with(|| DrawBuilder::new(GxPrimitive::Quads, 0));

    struct DisplacementVertex {
        position: Vec3,
        lightmap_coord: Vec2,
        texture_coords: [Vec2; 2],
        alpha: f32,
    }

    let x0 = 0.5;
    let x1 = face.lightmap_texture_size_in_luxels[0] as f32 + 0.5;
    let y0 = 0.5;
    let y1 = face.lightmap_texture_size_in_luxels[1] as f32 + 0.5;
    let corner_lightmap_coords: [Vec2; 4] =
        [vec2(x0, y0), vec2(x0, y1), vec2(x1, y1), vec2(x1, y0)];

    let mut position_indices = Vec::new();
    let mut vertex_color_indices = Vec::new();
    let mut lightmap_coordinates = Vec::new();
    let mut texture_coordinate1_indices = Vec::new();
    let mut texture_coordinate2_indices = Vec::new();
    let mut vertices = Vec::new();
    for y in 0..verts_per_side {
        for x in 0..verts_per_side {
            let xf = x as f32 / (verts_per_side - 1) as f32;
            let yf = y as f32 / (verts_per_side - 1) as f32;
            let vert = &verts[y * verts_per_side + x];
            let offset = vec3(vert.vec[0], vert.vec[1], vert.vec[2]) * vert.dist;
            let base_position = lerp(
                &lerp(
                    &corners[(closest_corner + 0) % 4],
                    &corners[(closest_corner + 3) % 4],
                    xf,
                ),
                &lerp(
                    &corners[(closest_corner + 1) % 4],
                    &corners[(closest_corner + 2) % 4],
                    xf,
                ),
                yf,
            );
            let lightmap_base_position = lerp(
                &lerp(&corner_lightmap_coords[0], &corner_lightmap_coords[3], xf),
                &lerp(&corner_lightmap_coords[1], &corner_lightmap_coords[2], xf),
                yf,
            );
            let displaced_position = base_position + offset;

            let (lightmap_s, lightmap_t) =
                if let (Some(lightmap), Some(lightmap_metadata)) = (lightmap, lightmap_metadata) {
                    let patch_s = lightmap_base_position.x;
                    let patch_t = lightmap_base_position.y;
                    let (patch_s, patch_t) = if lightmap_metadata.is_flipped {
                        (patch_t, patch_s)
                    } else {
                        (patch_s, patch_t)
                    };
                    (
                        patch_s / lightmap.width as f32,
                        patch_t / lightmap.height as f32,
                    )
                } else {
                    (0.0, 0.0)
                };

            let texture_s = tex_info.texture_vecs[0][0] * base_position.x
                + tex_info.texture_vecs[0][1] * base_position.y
                + tex_info.texture_vecs[0][2] * base_position.z
                + tex_info.texture_vecs[0][3];
            let texture_t = tex_info.texture_vecs[1][0] * base_position.x
                + tex_info.texture_vecs[1][1] * base_position.y
                + tex_info.texture_vecs[1][2] * base_position.z
                + tex_info.texture_vecs[1][3];

            let untransformed_texture_coordinate = vec3(
                texture_s / base_texture.width() as f32,
                texture_t / base_texture.height() as f32,
                1.0,
            );

            vertices.push(DisplacementVertex {
                position: displaced_position,
                lightmap_coord: vec2(lightmap_s, lightmap_t),
                texture_coords: [
                    texture_transform1 * untransformed_texture_coordinate,
                    texture_transform2 * untransformed_texture_coordinate,
                ],
                alpha: vert.alpha,
            });
        }
    }

    let min_tile_s1 = *vertices
        .iter()
        .map(|vertex| NotNan::new(vertex.texture_coords[0].x.floor()).unwrap())
        .min()
        .unwrap();
    let min_tile_t1 = *vertices
        .iter()
        .map(|vertex| NotNan::new(vertex.texture_coords[0].y.floor()).unwrap())
        .min()
        .unwrap();
    let min_tile_s2 = *vertices
        .iter()
        .map(|vertex| NotNan::new(vertex.texture_coords[1].x.floor()).unwrap())
        .min()
        .unwrap();
    let min_tile_t2 = *vertices
        .iter()
        .map(|vertex| NotNan::new(vertex.texture_coords[1].y.floor()).unwrap())
        .min()
        .unwrap();

    for y in 0..verts_per_side {
        for x in 0..verts_per_side {
            let index = verts_per_side * y + x;
            let vertex = &vertices[index];

            position_indices.push(positions.add_vertex(hashable_float(&vertex.position.data.0[0])));
            vertex_color_indices
                .push(vertex_colors.add_vertex([vertex.alpha.clamp(0.0, 255.0).round() as u8; 3]));
            lightmap_coordinates.push(quantize_lightmap_coord([
                vertex.lightmap_coord.x,
                vertex.lightmap_coord.y,
            ]));
            texture_coordinate1_indices.push(texture_coordinates.add_vertex(
                quantize_texture_coord([
                    vertex.texture_coords[0].x - min_tile_s1,
                    vertex.texture_coords[0].y - min_tile_t1,
                ]),
            ));
            texture_coordinate2_indices.push(texture_coordinates.add_vertex(
                quantize_texture_coord([
                    vertex.texture_coords[1].x - min_tile_s2,
                    vertex.texture_coords[1].y - min_tile_t2,
                ]),
            ));
        }
    }

    for y in 0..quads_per_side {
        for x in 0..quads_per_side {
            let mut data: Vec<u8> = Vec::new();
            let mut emit = |i: usize| {
                data.write_u16::<BigEndian>(position_indices[i]).unwrap();
                data.write_u16::<BigEndian>(vertex_color_indices[i])
                    .unwrap();
                data.write_u16::<BigEndian>(lightmap_coordinates[i][0])
                    .unwrap();
                data.write_u16::<BigEndian>(lightmap_coordinates[i][1])
                    .unwrap();
                data.write_u16::<BigEndian>(texture_coordinate1_indices[i])
                    .unwrap();
                data.write_u16::<BigEndian>(texture_coordinate2_indices[i])
                    .unwrap();
            };
            if (x ^ y) & 1 == 0 {
                emit(y * verts_per_side + x);
                emit((y + 1) * verts_per_side + x);
                emit((y + 1) * verts_per_side + x + 1);
                emit(y * verts_per_side + x + 1);
            } else {
                emit((y + 1) * verts_per_side + x);
                emit((y + 1) * verts_per_side + x + 1);
                emit(y * verts_per_side + x + 1);
                emit(y * verts_per_side + x);
            }
            display_list_builder.emit_vertices(4, &data);
        }
    }

    Ok(())
}

fn process_textured_brush_face(
    bsp: Bsp,
    asset_loader: &AssetLoader,
    ids: &mut TextureIdAllocator,
    positions: &mut AttributeBuilder<[FloatByBits; 3], u16>,
    normals: &mut AttributeBuilder<[u8; 3], u16>,
    texture_coords: &mut AttributeBuilder<[u16; 2], u16>,
    cluster_builder: &mut ClusterGeometryBuilder,
    lightmap: Option<&Lightmap>,
    face: &Face,
) -> Result<()> {
    let tex_info = &bsp.tex_infos()[face.tex_info as usize];
    if tex_info.tex_data == -1 {
        // Not textured.
        // TODO: Determine whether any such faces need to be drawn.
        return Ok(());
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
        Shader::LightmappedGeneric(LightmappedGeneric {
            base_texture_path, ..
        }) => {
            let base_texture = asset_loader.get_texture(base_texture_path)?;
            [base_texture.width() as f32, base_texture.height() as f32]
        }
        Shader::UnlitGeneric(UnlitGeneric {
            base_texture_path, ..
        }) => {
            let base_texture = asset_loader.get_texture(base_texture_path)?;
            [base_texture.width() as f32, base_texture.height() as f32]
        }
        Shader::WorldVertexTransition(WorldVertexTransition {
            base_texture_path, ..
        }) => {
            let base_texture = asset_loader.get_texture(base_texture_path)?;
            [base_texture.width() as f32, base_texture.height() as f32]
        }

        // Do not actually draw the special compile flag shaders.
        Shader::CompileSky => return Ok(()),

        shader => {
            eprintln!(
                "WARNING: Skipping shader in process_lit_textured_face: {}",
                shader.name(),
            );
            return Ok(());
        }
    };
    let packed_material =
        PackedMaterial::from_material(asset_loader, ids, &material, false)?.unwrap();
    let mut polygon_builder = PolygonBuilder::new(cluster_builder.draw_builder(
        Pass::from_material(&material, &packed_material),
        packed_material,
        ShaderParams::from_material(&material),
    ));

    let texture_transform = material.texture_transform();
    let face_vertices: Vec<Vertex> = bsp
        .iter_vertex_indices_from_face(face)
        .map(|vertex_index| {
            let mut vertex = convert_vertex(bsp, lightmap, face, tex_info, vertex_index);
            let texture_coord = texture_transform
                * vec3(
                    vertex.texture_coord[0] / base_texture_size[0],
                    vertex.texture_coord[1] / base_texture_size[1],
                    1.0,
                );
            vertex.texture_coord = [texture_coord[0], texture_coord[1]];
            vertex
        })
        .collect();

    let min_tile_s = *face_vertices
        .iter()
        .map(|vertex| NotNan::new(vertex.texture_coord[0].floor()).unwrap())
        .min()
        .unwrap();
    let min_tile_t = *face_vertices
        .iter()
        .map(|vertex| NotNan::new(vertex.texture_coord[1].floor()).unwrap())
        .min()
        .unwrap();

    for vertex in face_vertices {
        let position_index: u16 = positions.add_vertex(hashable_float(&vertex.position));
        let normal_index: u16 = normals.add_vertex(quantize_normal(vertex.normal));
        let lightmap_coord = quantize_lightmap_coord(vertex.lightmap_coord);
        let texture_coord_index: u16 = texture_coords.add_vertex(quantize_texture_coord([
            vertex.texture_coord[0] - min_tile_s,
            vertex.texture_coord[1] - min_tile_t,
        ]));

        polygon_builder.add_vertex((
            position_index,
            normal_index,
            lightmap_coord,
            texture_coord_index,
        ))?;
    }

    Ok(())
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
        let rounded = (coord[index] * 32768.0).round();
        let clamped = rounded.clamp(0.0, 65535.0);
        if rounded != clamped {
            eprintln!(
                "WARNING: Lightmap coord clamped from {} to {}",
                rounded, clamped,
            );
        }
        result[index] = clamped as u16;
    }
    result
}

fn quantize_texture_coord(coord: [f32; 2]) -> [u16; 2] {
    let mut result = [0; 2];
    for index in 0..2 {
        let rounded = (coord[index] * 256.0).round();
        let clamped = rounded.clamp(0.0, 65535.0);
        if rounded != clamped {
            eprintln!(
                "WARNING: Texture coord clamped from {} to {}",
                rounded, clamped,
            );
        }
        result[index] = clamped as u16;
    }
    result
}

fn pack_textures(
    asset_loader: &AssetLoader,
    map_geometry: &MapGeometry,
) -> Result<(Vec<TextureTableEntry>, Vec<u8>)> {
    fn get_dst_format(src_format: TextureFormat) -> Result<TextureFormat> {
        Ok(match src_format {
            TextureFormat::Dxt1 | TextureFormat::Dxt5 | TextureFormat::Rgba16f => {
                TextureFormat::GxTfCmpr
            }
            TextureFormat::Bgr8 | TextureFormat::Bgra8 | TextureFormat::Bgrx8 => {
                TextureFormat::GxTfRgba8
            }
            format => {
                panic!("unexpected texture format: {:?}", format)
            }
        })
    }

    fn limit_face_mips(texture: &Vtf, max_dimension: usize) -> Vec<VtfFaceMip> {
        let mut any_mip_matched = false;
        let mut smallest_face_mip = None;
        let mut limited_face_mips = Vec::new();
        for face_mip in texture.iter_face_mips() {
            smallest_face_mip = Some(face_mip);
            if face_mip.texture.width() <= max_dimension
                && face_mip.texture.height() <= max_dimension
            {
                any_mip_matched = true;
                limited_face_mips.push(face_mip);
            }
        }

        if !any_mip_matched {
            limited_face_mips.push(smallest_face_mip.unwrap());
        }

        limited_face_mips
    }

    const GAMECUBE_MEMORY_BUDGET: usize = 8 * 1024 * 1024;
    for max_dimension in [1024, 512, 256, 128, 64, 32, 16, 8] {
        let mut total_size = 0;
        for key in &map_geometry.texture_keys {
            match key {
                OwnedTextureKey::EncodeAsIs { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    let dst_format = get_dst_format(texture.format())?;
                    for face_mip in limit_face_mips(&texture, max_dimension) {
                        total_size += dst_format
                            .metrics()
                            .encoded_size(face_mip.texture.width(), face_mip.texture.height());
                    }
                }

                OwnedTextureKey::Intensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    let dst_format = TextureFormat::GxTfI8;
                    for face_mip in limit_face_mips(&texture, max_dimension) {
                        total_size += dst_format
                            .metrics()
                            .encoded_size(face_mip.texture.width(), face_mip.texture.height());
                    }
                }

                OwnedTextureKey::AlphaToIntensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    let dst_format = TextureFormat::GxTfI8;
                    for face_mip in limit_face_mips(&texture, max_dimension) {
                        total_size += dst_format
                            .metrics()
                            .encoded_size(face_mip.texture.width(), face_mip.texture.height());
                    }
                }

                OwnedTextureKey::ComposeIntensityAlpha {
                    intensity_texture_path,
                    alpha_texture_path,
                    ..
                } => {
                    let intensity_texture = asset_loader.get_texture(intensity_texture_path)?;
                    let alpha_texture = asset_loader.get_texture(alpha_texture_path)?;
                    if intensity_texture.width() == alpha_texture.width()
                        && intensity_texture.height() == alpha_texture.height()
                    {
                        assert_eq!(intensity_texture.mips().len(), alpha_texture.mips().len());

                        let dst_format = TextureFormat::GxTfIa8;
                        for face_mip in limit_face_mips(&intensity_texture, max_dimension) {
                            total_size += dst_format
                                .metrics()
                                .encoded_size(face_mip.texture.width(), face_mip.texture.height());
                        }
                    } else {
                        total_size += 32;
                    }
                }
            }
        }

        println!("Textures occupy {total_size} bytes with max dimension {max_dimension}");

        if total_size > GAMECUBE_MEMORY_BUDGET {
            continue;
        }

        let mut texture_table = Vec::new();
        let mut texture_data = Vec::new();

        let budgeted_size = total_size;
        total_size = 0;
        for key in &map_geometry.texture_keys {
            struct TextureMetadata {
                width: usize,
                height: usize,
                mip_count: usize,
                gx_flags: u8,
                gx_format: u8,
            }

            let start_offset = u32::try_from(texture_data.len()).unwrap();
            let metadata = match key {
                OwnedTextureKey::EncodeAsIs { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.face_count(), 1);

                    let dst_format = get_dst_format(texture.format())?;
                    let mut base_mip_size = None;
                    let mut mip_count = 0;
                    for face_mip in limit_face_mips(&texture, max_dimension) {
                        assert_eq!(face_mip.face, 0);
                        if base_mip_size.is_none() {
                            base_mip_size =
                                Some((face_mip.texture.width(), face_mip.texture.height()));
                        }
                        texture_data.extend_from_slice(
                            TextureBuf::transcode(face_mip.texture.as_slice(), dst_format).data(),
                        );
                        mip_count += 1;
                    }

                    TextureMetadata {
                        width: base_mip_size.unwrap().0,
                        height: base_mip_size.unwrap().1,
                        mip_count,
                        gx_flags: gx_texture_flags(texture.flags()),
                        gx_format: gx_texture_format(dst_format),
                    }
                }

                OwnedTextureKey::Intensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.face_count(), 1);

                    let dst_format = TextureFormat::GxTfI8;
                    let mut base_mip_size = None;
                    let mut mip_count = 0;
                    for face_mip in limit_face_mips(&texture, max_dimension) {
                        assert_eq!(face_mip.face, 0);
                        if base_mip_size.is_none() {
                            base_mip_size =
                                Some((face_mip.texture.width(), face_mip.texture.height()));
                        }
                        texture_data.extend_from_slice(
                            TextureBuf::transcode(face_mip.texture.as_slice(), dst_format).data(),
                        );
                        mip_count += 1;
                    }

                    TextureMetadata {
                        width: base_mip_size.unwrap().0,
                        height: base_mip_size.unwrap().1,
                        mip_count,
                        gx_flags: gx_texture_flags(texture.flags()),
                        gx_format: gx_texture_format(dst_format),
                    }
                }

                OwnedTextureKey::AlphaToIntensity { texture_path } => {
                    let texture = asset_loader.get_texture(texture_path)?;
                    assert_eq!(texture.face_count(), 1);
                    let dst_format = TextureFormat::GxTfI8;

                    let mut base_mip_size = None;
                    let mut mip_count = 0;
                    for face_mip in limit_face_mips(&texture, max_dimension) {
                        assert_eq!(face_mip.face, 0);
                        if base_mip_size.is_none() {
                            base_mip_size =
                                Some((face_mip.texture.width(), face_mip.texture.height()));
                        }

                        // Broadcast alpha to all channels.
                        let mut texel_data = TextureBuf::transcode(
                            face_mip.texture.as_slice(),
                            TextureFormat::Rgba8,
                        )
                        .into_data();
                        for texel_index in 0..face_mip.texture.width() * face_mip.texture.height() {
                            let offset = 4 * texel_index;
                            texel_data[offset] = texel_data[offset + 3];
                            texel_data[offset + 1] = texel_data[offset + 3];
                            texel_data[offset + 2] = texel_data[offset + 3];
                        }

                        texture_data.extend_from_slice(
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
                        );

                        mip_count += 1;
                    }

                    TextureMetadata {
                        width: base_mip_size.unwrap().0,
                        height: base_mip_size.unwrap().1,
                        mip_count,
                        gx_flags: gx_texture_flags(texture.flags()),
                        gx_format: gx_texture_format(dst_format),
                    }
                }

                OwnedTextureKey::ComposeIntensityAlpha {
                    intensity_texture_path,
                    intensity_from_alpha,
                    alpha_texture_path,
                } => {
                    let intensity_texture = asset_loader.get_texture(intensity_texture_path)?;
                    let alpha_texture = asset_loader.get_texture(alpha_texture_path)?;
                    if intensity_texture.width() == alpha_texture.width()
                        && intensity_texture.height() == alpha_texture.height()
                    {
                        assert_eq!(intensity_texture.flags() & 0xc, alpha_texture.flags() & 0xc);
                        assert_eq!(intensity_texture.mips().len(), alpha_texture.mips().len());
                        assert_eq!(intensity_texture.face_count(), 1);
                        assert_eq!(alpha_texture.face_count(), 1);
                        let dst_format = TextureFormat::GxTfIa8;

                        let mut base_mip_size = None;
                        let mut mip_count = 0;
                        let intensity_face_mips =
                            limit_face_mips(&intensity_texture, max_dimension);
                        let alpha_face_mips = limit_face_mips(&alpha_texture, max_dimension);
                        assert_eq!(intensity_face_mips.len(), alpha_face_mips.len());
                        for index in 0..intensity_face_mips.len() {
                            let intensity_face_mip = intensity_face_mips[index];
                            let alpha_face_mip = alpha_face_mips[index];
                            assert_eq!(intensity_face_mip.face, 0);
                            assert_eq!(alpha_face_mip.face, 0);
                            assert_eq!(intensity_face_mip.mip_level, alpha_face_mip.mip_level);
                            let width = intensity_face_mip.texture.width();
                            let height = intensity_face_mip.texture.height();

                            if base_mip_size.is_none() {
                                base_mip_size = Some((
                                    intensity_face_mip.texture.width(),
                                    intensity_face_mip.texture.height(),
                                ));
                            }

                            // Combine the intensity and alpha textures by channel into a new
                            // texture.
                            let intensity_data = TextureBuf::transcode(
                                intensity_face_mip.texture.as_slice(),
                                TextureFormat::Rgba8,
                            )
                            .into_data();
                            let alpha_data = TextureBuf::transcode(
                                alpha_face_mip.texture.as_slice(),
                                TextureFormat::Rgba8,
                            )
                            .into_data();
                            let mut texels = Vec::with_capacity(4 * width * height);
                            for texel_index in 0..width * height {
                                let offset = 4 * texel_index;
                                texels.extend_from_slice(&if *intensity_from_alpha {
                                    [
                                        intensity_data[offset + 3],
                                        intensity_data[offset + 3],
                                        intensity_data[offset + 3],
                                        alpha_data[offset + 3],
                                    ]
                                } else {
                                    [
                                        intensity_data[offset],
                                        intensity_data[offset + 1],
                                        intensity_data[offset + 2],
                                        alpha_data[offset + 3],
                                    ]
                                });
                            }

                            texture_data.extend_from_slice(
                                TextureBuf::transcode(
                                    TextureBuf::new(TextureFormat::Rgba8, width, height, texels)
                                        .as_slice(),
                                    dst_format,
                                )
                                .data(),
                            );
                            mip_count += 1;
                        }

                        TextureMetadata {
                            width: base_mip_size.unwrap().0,
                            height: base_mip_size.unwrap().1,
                            mip_count,
                            gx_flags: gx_texture_flags(intensity_texture.flags()),
                            gx_format: gx_texture_format(dst_format),
                        }
                    } else {
                        // Skip for now.
                        // TODO: Scale to the maximum dimension.
                        eprintln!("WARNING: Skipping ComposeIntensityAlpha for textures with different dimensions: {}, {}",
                        intensity_texture_path,
                        alpha_texture_path,
                    );

                        texture_data.extend_from_slice(&[0; 32]);
                        TextureMetadata {
                            width: 8,
                            height: 8,
                            mip_count: 1,
                            gx_flags: 0,
                            gx_format: gx_texture_format(TextureFormat::GxTfCmpr),
                        }
                    }
                }
            };

            let end_offset = u32::try_from(texture_data.len()).unwrap();

            // Write a texture table entry.
            texture_table.push(TextureTableEntry {
                width: metadata.width as u16,
                height: metadata.height as u16,
                mip_count: metadata.mip_count as u8,
                flags: metadata.gx_flags,
                format: metadata.gx_format,
                _padding: 0,
                start_offset,
                end_offset,
            });
            total_size += (end_offset - start_offset) as usize;
        }

        assert_eq!(total_size, budgeted_size);

        return Ok((texture_table, texture_data));
    }
    bail!("Unable to fit textures within the memory budget.");
}

fn gx_texture_flags(vtf_flags: u32) -> u8 {
    let wrap_s = (vtf_flags & 0x4) >> 2; // 0x01
    let wrap_t = (vtf_flags & 0x8) >> 2; // 0x02
    (wrap_s | wrap_t) as u8
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

fn pack_brush_geometry(
    map_geometry: &MapGeometry,
    texture_table: &[TextureTableEntry],
) -> (
    Vec<ClusterGeometryTableEntry>,
    Vec<u32>,
    Vec<u8>,
    Vec<ClusterGeometryReferencesEntry>,
) {
    // TODO: Transpose this table for potential cache friendliness?

    let mut cluster_geometry_table = Vec::new();
    let mut cluster_geometry_byte_code = Vec::new();
    let mut cluster_geometry_display_lists = Vec::new();
    let mut cluster_geometry_references = Vec::new();

    for cluster in &map_geometry.clusters {
        let mut cluster_geometry_table_entry = ClusterGeometryTableEntry {
            byte_code_index_ranges: [[0, 0]; 6],
        };
        let mut display_list_offset = u32::try_from(cluster_geometry_display_lists.len()).unwrap();
        for mode in 0..6 {
            cluster_geometry_table_entry.byte_code_index_ranges[mode as usize][0] =
                u32::try_from(cluster_geometry_byte_code.len()).unwrap();

            let mut prev_base_map_id = None;
            let mut prev_aux_map_id = None;
            let mut prev_alpha = None;
            for ((pass, material, params), draw_display_list) in
                &cluster.display_lists_by_pass_material_params
            {
                let mut display_list = DisplayList::new();
                if pass.as_mode() == mode {
                    // Bind the base texture as TEXMAP1 using TEXCOORD1.
                    if prev_base_map_id != Some(material.base_id) {
                        prev_base_map_id = Some(material.base_id);
                        display_list.append_bind_texture(1, material.base_id, texture_table);
                        display_list.append_texcoord_scale(1, material.base_id, texture_table);
                    }

                    // Bind the aux texture as TEXMAP2 reusing TEXCOORD1.
                    if let Some(aux_id) = material.aux_id {
                        if prev_aux_map_id != Some(aux_id) {
                            prev_aux_map_id = Some(aux_id);
                            display_list.append_bind_texture(2, aux_id, texture_table);
                            // NOTE: Assume the aux texture has the same dimensions as the base
                            // texture. They share texture coordinate 1 so there's no need to set
                            // the scale again.
                        }
                    }

                    if prev_alpha != Some(params.alpha) {
                        prev_alpha = Some(params.alpha);

                        let z_comp_before_tex = match params.alpha {
                            ShaderParamsAlpha::AlphaTest { .. } => 0,
                            _ => 1,
                        };
                        let compare_type = match params.alpha {
                            ShaderParamsAlpha::AlphaTest { .. } => {
                                BytecodeOp::ALPHA_COMPARE_TYPE_GEQUAL
                            }
                            _ => BytecodeOp::ALPHA_COMPARE_TYPE_ALWAYS,
                        };
                        let reference = match params.alpha {
                            ShaderParamsAlpha::AlphaTest { threshold } => threshold,
                            _ => 0,
                        };
                        BytecodeOp::SetAlphaCompare {
                            z_comp_before_tex,
                            compare_type,
                            reference,
                        }
                        .append_to(&mut cluster_geometry_byte_code);
                    }

                    display_list
                        .commands
                        .extend_from_slice(&draw_display_list.commands);
                    display_list.pad_to_alignment();
                    display_list
                        .write_to(
                            &mut cluster_geometry_display_lists,
                            |cluster_geometry_display_lists, reference| {
                                cluster_geometry_references.push(ClusterGeometryReferencesEntry {
                                    display_list_offset: cluster_geometry_display_lists
                                        .len()
                                        .try_into()
                                        .unwrap(),
                                    texture_id: match reference {
                                        gx::display_list::Reference::Texture(x) => x,
                                    },
                                    _padding: 0,
                                });
                            },
                        )
                        .unwrap();
                    let next_display_list_offset =
                        u32::try_from(cluster_geometry_display_lists.len()).unwrap();
                    assert_eq!(next_display_list_offset & 31, 0);
                    let display_list_size = next_display_list_offset - display_list_offset;
                    assert_eq!(display_list_size & 31, 0);

                    BytecodeOp::Draw {
                        display_list_offset,
                        display_list_size,
                    }
                    .append_to(&mut cluster_geometry_byte_code);

                    display_list_offset = next_display_list_offset;
                }
            }

            cluster_geometry_table_entry.byte_code_index_ranges[mode as usize][1] =
                u32::try_from(cluster_geometry_byte_code.len()).unwrap();
        }
        cluster_geometry_table.push(cluster_geometry_table_entry);
    }

    (
        cluster_geometry_table,
        cluster_geometry_byte_code,
        cluster_geometry_display_lists,
        cluster_geometry_references,
    )
}

fn pack_displacement_geometry(
    map_geometry: &MapGeometry,
    texture_table: &[TextureTableEntry],
) -> (
    Vec<DisplacementTableEntry>,
    Vec<u32>,
    Vec<u8>,
    Vec<DisplacementReferencesEntry>,
) {
    let mut displacement_table = Vec::new();
    let mut displacement_byte_code = Vec::new();
    let mut displacement_display_lists = Vec::new();
    let mut displacement_references = Vec::new();

    for mode in 0..2 {
        let byte_code_start_index = u32::try_from(displacement_byte_code.len()).unwrap();

        for ((pass, face_index, packed_material), draw_display_list) in
            &map_geometry.displacement_display_lists_by_pass_face_material
        {
            if pass.as_mode() != mode {
                continue;
            }

            let display_list_offset = u32::try_from(displacement_display_lists.len()).unwrap();
            let mut display_list = DisplayList::new();

            BytecodeOp::SetFaceIndex {
                face_index: *face_index,
            }
            .append_to(&mut displacement_byte_code);

            // Bind the base texture to TEXMAP1 with TEXCOORD1.
            display_list.append_bind_texture(1, packed_material.base_id, texture_table);
            display_list.append_texcoord_scale(1, packed_material.base_id, texture_table);

            if *pass == DisplacementPass::WorldVertexTransition {
                // Bind the aux texture to TEXMAP2 with TEXCOORD2.
                let aux_id = packed_material.aux_id.unwrap();
                display_list.append_bind_texture(2, aux_id, texture_table);
                display_list.append_texcoord_scale(2, aux_id, texture_table);
            }

            display_list
                .commands
                .extend_from_slice(&draw_display_list.commands);
            display_list.pad_to_alignment();
            display_list
                .write_to(
                    &mut displacement_display_lists,
                    |displacement_display_lists, reference| {
                        displacement_references.push(DisplacementReferencesEntry {
                            display_list_offset: displacement_display_lists
                                .len()
                                .try_into()
                                .unwrap(),
                            texture_id: match reference {
                                gx::display_list::Reference::Texture(x) => x,
                            },
                            _padding: 0,
                        });
                    },
                )
                .unwrap();
            let next_display_list_offset = u32::try_from(displacement_display_lists.len()).unwrap();
            assert_eq!(next_display_list_offset & 31, 0);
            let display_list_size = next_display_list_offset - display_list_offset;
            assert_eq!(display_list_size & 31, 0);

            BytecodeOp::Draw {
                display_list_offset,
                display_list_size,
            }
            .append_to(&mut displacement_byte_code);
        }

        let byte_code_end_index = u32::try_from(displacement_byte_code.len()).unwrap();
        displacement_table.push(DisplacementTableEntry {
            byte_code_start_index,
            byte_code_end_index,
        });
    }

    (
        displacement_table,
        displacement_byte_code,
        displacement_display_lists,
        displacement_references,
    )
}

fn pack_bsp_nodes(bsp: Bsp) -> Vec<BspNode> {
    let mut bsp_nodes = Vec::new();
    for node in bsp.nodes() {
        let plane = &bsp.planes()[node.planenum as usize];
        bsp_nodes.push(BspNode {
            plane: [
                plane.normal[0],
                plane.normal[1],
                plane.normal[2],
                plane.dist,
            ],
            children: node.children,
        });
    }
    bsp_nodes
}

fn pack_bsp_leaves(bsp: Bsp) -> Vec<BspLeaf> {
    let mut bsp_leaves = Vec::new();
    for leaf in bsp.leaves() {
        bsp_leaves.push(BspLeaf {
            cluster: leaf.cluster(),
        });
    }
    bsp_leaves
}

fn pack_visibility(bsp: Bsp) -> Vec<u8> {
    // Scan each vis chunk to determine its length.
    let mut sized_vis_chunks = Vec::new();
    for cluster in bsp.visibility().iter_clusters() {
        sized_vis_chunks.push(cluster.find_data());
    }

    // Build the index.
    let mut offset = 4 * sized_vis_chunks.len() as u32 + 4;
    let mut visibility = Vec::new();
    visibility
        .write_u32::<BigEndian>(sized_vis_chunks.len() as u32)
        .unwrap();
    for &chunk in &sized_vis_chunks {
        visibility.write_u32::<BigEndian>(offset).unwrap();
        offset += chunk.len() as u32;
    }

    // Append all chunks.
    for chunk in sized_vis_chunks {
        visibility.extend_from_slice(chunk);
    }

    visibility
}

fn pack_lightmaps(
    bsp: Bsp,
    cluster_lightmaps: &HashMap<i16, Lightmap>,
    displacement_lightmaps: &HashMap<u16, Lightmap>,
) -> (
    Vec<ClusterLightmapTableEntry>,
    Vec<DisplacementLightmapTableEntry>,
    Vec<LightmapPatchTableEntry>,
    Vec<u8>,
) {
    let mut lightmap_cluster_table = Vec::new();
    let mut lightmap_patch_table = Vec::new();
    let mut lightmap_data = Vec::new();

    let cluster_end_index = cluster_lightmaps.keys().copied().max().unwrap();
    for cluster_index in 0..cluster_end_index {
        if let Some(lightmap) = &cluster_lightmaps.get(&cluster_index) {
            let patch_table_start_index = u32::try_from(lightmap_patch_table.len()).unwrap();
            pack_cluster_lightmap_patches(
                bsp,
                cluster_index,
                lightmap,
                &mut lightmap_patch_table,
                &mut lightmap_data,
            );
            let patch_table_end_index = u32::try_from(lightmap_patch_table.len()).unwrap();

            lightmap_cluster_table.push(ClusterLightmapTableEntry {
                common: CommonLightmapTableEntry {
                    width: lightmap.width as u16,
                    height: lightmap.height as u16,
                    patch_table_start_index,
                    patch_table_end_index,
                },
            });
        } else {
            lightmap_cluster_table.push(ClusterLightmapTableEntry {
                common: CommonLightmapTableEntry {
                    width: 0,
                    height: 0,
                    patch_table_start_index: 0,
                    patch_table_end_index: 0,
                },
            });
        }
    }

    let lightmap_displacement_table = pack_displacement_lightmap_patches(
        bsp,
        displacement_lightmaps,
        &mut lightmap_patch_table,
        &mut lightmap_data,
    );

    (
        lightmap_cluster_table,
        lightmap_displacement_table,
        lightmap_patch_table,
        lightmap_data,
    )
}

fn pack_cluster_lightmap_patches(
    bsp: Bsp,
    cluster_index: i16,
    lightmap: &Lightmap,
    lightmap_patch_table: &mut Vec<LightmapPatchTableEntry>,
    lightmap_data: &mut Vec<u8>,
) {
    for leaf in bsp.iter_worldspawn_leaves() {
        if leaf.cluster() != cluster_index {
            continue;
        }

        let mut lightmap_patches_by_data_offset = HashMap::new();
        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs == -1 || face.tex_info == -1 {
                continue;
            }
            lightmap_patches_by_data_offset
                .entry(face.light_ofs)
                .or_insert(lightmap_patch_from_face(
                    bsp,
                    face,
                    lightmap.metadata_by_data_offset[&face.light_ofs],
                ));
        }

        pack_lightmap_patches(
            bsp,
            lightmap_patches_by_data_offset,
            lightmap_patch_table,
            lightmap_data,
        );
    }
}

fn pack_displacement_lightmap_patches(
    bsp: Bsp,
    displacement_lightmaps: &HashMap<u16, Lightmap>,
    lightmap_patch_table: &mut Vec<LightmapPatchTableEntry>,
    lightmap_data: &mut Vec<u8>,
) -> Vec<DisplacementLightmapTableEntry> {
    let mut lightmap_displacement_table = Vec::new();
    for disp_info in bsp.disp_infos() {
        let lightmap = match displacement_lightmaps.get(&disp_info.map_face) {
            Some(lightmap) => lightmap,
            None => continue,
        };
        let face = &bsp.faces()[disp_info.map_face as usize];

        let patch_table_start_index = u32::try_from(lightmap_patch_table.len()).unwrap();
        let mut lightmap_patches_by_data_offset = HashMap::new();
        let metadata = lightmap.metadata_by_data_offset[&face.light_ofs];
        lightmap_patches_by_data_offset
            .entry(face.light_ofs)
            .or_insert(lightmap_patch_from_face(bsp, face, metadata));
        pack_lightmap_patches(
            bsp,
            lightmap_patches_by_data_offset,
            lightmap_patch_table,
            lightmap_data,
        );
        let patch_table_end_index = u32::try_from(lightmap_patch_table.len()).unwrap();

        lightmap_displacement_table.push(DisplacementLightmapTableEntry {
            face_index: disp_info.map_face,
            _padding: 0,
            common: CommonLightmapTableEntry {
                width: lightmap.width as u16,
                height: lightmap.height as u16,
                patch_table_start_index,
                patch_table_end_index,
            },
        });
    }
    lightmap_displacement_table
}

fn lightmap_patch_from_face(bsp: Bsp, face: &Face, metadata: LightmapMetadata) -> LightmapPatch {
    let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
    let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;
    let tex_info = &bsp.tex_infos()[face.tex_info as usize];
    let style_count: u8 = face
        .styles
        .iter()
        .map(|&x| if x != 255 { 1 } else { 0 })
        .sum();
    assert!(style_count > 0);
    let bump_light = (tex_info.flags & 0x800) != 0;

    LightmapPatch {
        width: u8::try_from(width).unwrap(),
        height: u8::try_from(height).unwrap(),
        style_count,
        bump_light,
        luxel_offset: metadata.luxel_offset,
        is_flipped: metadata.is_flipped,
    }
}

fn pack_lightmap_patches(
    bsp: Bsp,
    lightmap_patches_by_data_offset: HashMap<i32, LightmapPatch>,
    lightmap_patch_table: &mut Vec<LightmapPatchTableEntry>,
    lightmap_data: &mut Vec<u8>,
) {
    let mut data_offsets: Vec<_> = lightmap_patches_by_data_offset.keys().copied().collect();
    data_offsets.sort_unstable();
    for data_offset in data_offsets {
        let data_start_offset = u32::try_from(lightmap_data.len()).unwrap();

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
            let patch_index = (angle_count * (patch.style_count - style - 1) + angle) as usize;
            let patch_base = data_offset as usize + patch_size * patch_index;

            // Traverse blocks in texture format order.
            for coarse_y in 0..blocks_high {
                for coarse_x in 0..blocks_wide {
                    lightmap_data.extend_from_slice(
                        &transcode_lightmap_patch_to_gamecube_cmpr_sub_block(
                            bsp,
                            patch,
                            patch_base,
                            4 * coarse_x,
                            4 * coarse_y,
                        ),
                    );
                }
            }
        }
        let data_end_offset = u32::try_from(lightmap_data.len()).unwrap();

        lightmap_patch_table.push(LightmapPatchTableEntry {
            sub_block_x: (patch.luxel_offset[0] / 4) as u8,
            sub_block_y: (patch.luxel_offset[1] / 4) as u8,
            sub_blocks_wide: blocks_wide as u8,
            sub_blocks_high: blocks_high as u8,
            style_count: patch.style_count,
            _padding1: 0,
            _padding2: 0,
            data_start_offset,
            data_end_offset,
        });
    }
}

fn transcode_lightmap_patch_to_gamecube_cmpr_sub_block(
    bsp: Bsp,
    patch: &LightmapPatch,
    patch_base: usize,
    x0: usize,
    y0: usize,
) -> [u8; 8] {
    // Gather a 4x4 block of texels.
    let mut texels = Vec::with_capacity(64);
    for fine_y in 0..4 {
        for fine_x in 0..4 {
            let dst_x = x0 + fine_x;
            let dst_y = y0 + fine_y;
            let (src_x, src_y) = if patch.is_flipped {
                (dst_y, dst_x)
            } else {
                (dst_x, dst_y)
            };
            // Clamp source coordinates to smear the last row/column into unused space. This should
            // be more friendly to DXT1 encoding, avoiding arbitrary additional colors.
            let src_x = src_x.min(patch.width as usize);
            let src_y = src_y.min(patch.height as usize);
            let src_offset =
                patch_base + 4 * (patch.width as usize * src_y as usize + src_x as usize);
            let rgb = bsp.lighting().at_offset(src_offset, 1)[0].to_srgb8();
            texels.extend_from_slice(&rgb);
            texels.push(255);
        }
    }

    // Transcode to GX_TF_CMPR. Note that there is an 8x8 encoding block size, composed of four 4x4
    // permuted DXT1 blocks. The three padding sub-blocks are discarded.
    TextureBuf::transcode(
        TextureBuf::new(TextureFormat::Rgba8, 4, 4, texels).as_slice(),
        TextureFormat::GxTfCmpr,
    )
    .into_data()[..8]
        .try_into()
        .unwrap()
}
