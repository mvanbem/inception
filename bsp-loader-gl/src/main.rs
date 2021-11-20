#![deny(unsafe_op_in_unsafe_fn, unused_unsafe)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::iter::repeat_with;
use std::time::{Duration, Instant};

use anyhow::Result;
use byteorder::{BigEndian, WriteBytesExt};
use glium::glutin::dpi::LogicalSize;
use glium::glutin::event::{
    DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent,
};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::index::PrimitiveType;
use glium::texture::compressed_srgb_texture2d::CompressedSrgbTexture2d;
use glium::texture::{ClientFormat, CompressedMipmapsOption, CompressedSrgbFormat, RawImage2d};
use glium::{
    implement_vertex, uniform, BackfaceCullingMode, Depth, DepthTest, Display, DrawParameters,
    IndexBuffer, Program, Surface, VertexBuffer,
};
use nalgebra_glm::{look_at, perspective, radians, rotate, translate, vec1, vec3};

use crate::bsp::{Bsp, ClusterIndex};
use crate::display_list::DisplayListBuilder;
use crate::texture_atlas::{RgbU8Image, RgbU8TextureAtlas};

mod bsp;
mod display_list;
mod texture_atlas;
mod transmute_utils;

#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    lightmap_blend: f32,
    lightmap_coord: [f32; 2],
}

implement_vertex!(Vertex, position, lightmap_blend, lightmap_coord);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct VertexKey {
    face: *const bsp::Face,
    vertex_index: usize,
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

fn main() -> Result<()> {
    let bsp_data = std::fs::read("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Half-Life 2\\hl2\\maps\\d1_trainstation_01.bsp")?;
    let bsp = Bsp::new(&bsp_data);

    let (lightmap_image, lightmap_metadata_by_data_offset) = build_lightmaps(bsp)?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut emitted_vertices_by_source = HashMap::new();
    let mut gamecube_position_indices = HashMap::new();
    let mut gamecube_texcoord_indices = HashMap::new();
    let mut next_gamecube_position_index = 0;
    let mut next_gamecube_texcoord_index = 0;
    let mut gamecube_position_data = Vec::new();
    let mut gamecube_texcoord_data = Vec::new();
    let mut cluster_display_lists = repeat_with(|| DisplayListBuilder::new())
        .take(bsp.leaves().len())
        .collect::<Vec<_>>();
    for (_leaf_index, leaf) in bsp.enumerate_worldspawn_leaves() {
        if leaf.cluster == -1 {
            // Leaf is not potentially visible from anywhere.
            continue;
        }
        let mut gamecube_batch_builder =
            cluster_display_lists[leaf.cluster as usize].build_batch(DisplayListBuilder::TRIANGLES);

        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs == -1 || face.tex_info == -1 {
                // Not a textured lightmapped surface.
                continue;
            }
            let lightmap_metadata = &lightmap_metadata_by_data_offset[&face.light_ofs];
            let tex_info = &bsp.tex_infos()[face.tex_info as usize];

            let mut first_index = None;
            let mut first_gamecube_position_index = None;
            let mut first_gamecube_texcoord_index = None;
            let mut prev_index = None;
            let mut prev_gamecube_position_index = None;
            let mut prev_gamecube_texcoord_index = None;
            for vertex_index in bsp.iter_vertex_indices_from_face(face) {
                let key = VertexKey { face, vertex_index };
                let remapped_index = if emitted_vertices_by_source.contains_key(&key) {
                    *emitted_vertices_by_source.get(&key).unwrap()
                } else {
                    let vertex = convert_vertex(
                        bsp,
                        vertex_index,
                        tex_info,
                        face,
                        lightmap_metadata,
                        &lightmap_image,
                    );

                    // Emit the vertex.
                    let remapped_index = u16::try_from(vertices.len()).unwrap();
                    vertices.push(vertex);
                    emitted_vertices_by_source.insert(key, remapped_index);
                    remapped_index
                };

                let vertex = &vertices[remapped_index as usize];
                let gamecube_position_index = {
                    let gamecube_position = vertex.position;
                    let key = [
                        FloatByBits(gamecube_position[0]),
                        FloatByBits(gamecube_position[1]),
                        FloatByBits(gamecube_position[2]),
                    ];
                    if gamecube_position_indices.contains_key(&key) {
                        gamecube_position_indices.get(&key).copied().unwrap()
                    } else {
                        let gamecube_position_index = next_gamecube_position_index;
                        next_gamecube_position_index += 1;
                        gamecube_position_data.write_f32::<BigEndian>(gamecube_position[0])?;
                        gamecube_position_data.write_f32::<BigEndian>(gamecube_position[1])?;
                        gamecube_position_data.write_f32::<BigEndian>(gamecube_position[2])?;
                        gamecube_position_indices.insert(key, gamecube_position_index);
                        gamecube_position_index
                    }
                };
                let gamecube_texcoord_index = {
                    let gamecube_texcoord = vertex.lightmap_coord;
                    let key = [
                        FloatByBits(gamecube_texcoord[0]),
                        FloatByBits(gamecube_texcoord[1]),
                    ];
                    if gamecube_texcoord_indices.contains_key(&key) {
                        gamecube_texcoord_indices.get(&key).copied().unwrap()
                    } else {
                        let gamecube_texcoord_index = next_gamecube_texcoord_index;
                        next_gamecube_texcoord_index += 1;
                        gamecube_texcoord_data.write_f32::<BigEndian>(gamecube_texcoord[0])?;
                        gamecube_texcoord_data.write_f32::<BigEndian>(gamecube_texcoord[1])?;
                        gamecube_texcoord_indices.insert(key, gamecube_texcoord_index);
                        gamecube_texcoord_index
                    }
                };

                if first_index.is_none() {
                    first_index = Some(remapped_index);
                    first_gamecube_position_index = Some(gamecube_position_index);
                    first_gamecube_texcoord_index = Some(gamecube_texcoord_index);
                }

                if let Some(prev_index) = prev_index {
                    indices.push(first_index.unwrap());
                    indices.push(prev_index);
                    indices.push(remapped_index);

                    let mut data = [0; 12];
                    let mut w = &mut data[..];
                    w.write_u16::<BigEndian>(first_gamecube_position_index.unwrap())?;
                    w.write_u16::<BigEndian>(first_gamecube_texcoord_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_gamecube_position_index.unwrap())?;
                    w.write_u16::<BigEndian>(prev_gamecube_texcoord_index.unwrap())?;
                    w.write_u16::<BigEndian>(gamecube_position_index)?;
                    w.write_u16::<BigEndian>(gamecube_texcoord_index)?;
                    gamecube_batch_builder.emit_vertices(3, &data);
                }
                prev_index = Some(remapped_index);
                prev_gamecube_position_index = Some(gamecube_position_index);
                prev_gamecube_texcoord_index = Some(gamecube_texcoord_index);
            }
        }
    }

    // Scan visibility data.
    for (src_index, cluster) in bsp.visibility().iter_clusters().enumerate() {
        let _src_index = ClusterIndex(src_index);
        for _visible_cluster in cluster.iter_visible_clusters() {
            // todo
        }
    }

    if true {
        lightmap_image.write_to_png("lightmap_atlas.png")?;
    }

    if true {
        {
            let mut f = BufWriter::new(File::create("position_data.dat")?);
            f.write_all(&gamecube_position_data)?;
            f.flush()?;
        }
        {
            let mut f = BufWriter::new(File::create("texcoord_data.dat")?);
            f.write_all(&gamecube_texcoord_data)?;
            f.flush()?;
        }
        {
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

            let mut f = BufWriter::new(File::create("display_lists.dat")?);
            f.write_all(&index)?;
            for display_list in built_display_lists {
                f.write_all(&display_list)?;
            }
            f.flush()?;
        }
        {
            // struct BspNode {
            //     plane: [f32; 4],
            //     children: [i32; 2],
            // }
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

            let mut f = BufWriter::new(File::create("bsp_nodes.dat")?);
            f.write_all(&data)?;
            f.flush()?;
        }
        {
            // struct BspLeaf {
            //     cluster: i16,
            // }
            let mut data = Vec::new();
            for leaf in bsp.leaves() {
                data.write_i16::<BigEndian>(leaf.cluster).unwrap();
            }

            let mut f = BufWriter::new(File::create("bsp_leaves.dat")?);
            f.write_all(&data)?;
            f.flush()?;
        }
        {
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

            let mut f = BufWriter::new(File::create("vis.dat")?);
            f.write_all(&index)?;
            for &chunk in &sized_vis_chunks {
                f.write_all(chunk)?;
            }
            f.flush()?;
        }
    }

    let events_loop = EventLoop::new();
    let display = Display::new(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024.0, 768.0))
            .with_title("bsp-loader-gl"),
        glium::glutin::ContextBuilder::new(),
        &events_loop,
    )
    .unwrap();

    const VERTEX_SHADER_SOURCE: &str = r#"
        #version 330

        uniform mat4 mvp_matrix;

        in vec3 position;
        in float lightmap_blend;
        in vec2 lightmap_coord;

        out float interpolated_lightmap_blend;
        out vec2 interpolated_lightmap_coord;

        void main() {
            gl_Position = mvp_matrix * vec4(position, 1.0);
            interpolated_lightmap_blend = lightmap_blend;
            interpolated_lightmap_coord = lightmap_coord;
        }
    "#;
    const FRAGMENT_SHADER_SOURCE: &str = r#"
        #version 330

        uniform sampler2D lightmap;

        in float interpolated_lightmap_blend;
        in vec2 interpolated_lightmap_coord;

        out vec4 rendered_color;

        void main() {
            vec4 plain_color = vec4(1.0, 0.0, 1.0, 1.0);
            vec4 lightmap_color = vec4(texture(lightmap, interpolated_lightmap_coord).rgb, 1.0);
            // vec4 lightmap_color = vec4(interpolated_lightmap_coord.st, 0.0, 1.0);
            rendered_color = mix(plain_color, lightmap_color, interpolated_lightmap_blend);
        }
    "#;
    let program =
        Program::from_source(&display, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE, None)?;

    let vertex_buffer = VertexBuffer::new(&display, &vertices)?;
    let index_buffer = IndexBuffer::new(&display, PrimitiveType::TrianglesList, &indices)?;
    let lightmap_texture = CompressedSrgbTexture2d::with_format(
        &display,
        RawImage2d {
            data: Cow::Borrowed(lightmap_image.data()),
            width: lightmap_image.width() as u32,
            height: lightmap_image.height() as u32,
            format: ClientFormat::U8U8U8,
        },
        CompressedSrgbFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
    )?;

    let mut dragging = false;
    let mut held_keys: HashMap<VirtualKeyCode, bool> = [
        VirtualKeyCode::W,
        VirtualKeyCode::S,
        VirtualKeyCode::A,
        VirtualKeyCode::D,
        VirtualKeyCode::Space,
        VirtualKeyCode::LControl,
        VirtualKeyCode::LShift,
    ]
    .into_iter()
    .map(|code| (code, false))
    .collect();
    let mut pos = vec3(0.0, 0.0, 0.0);
    let mut yaw = std::f32::consts::PI;
    let mut pitch: f32 = 0.0;
    let mut last_timestamp = Instant::now();

    events_loop.run(move |event, _target, control_flow| match event {
        Event::NewEvents(_) => {
            let now = Instant::now();
            let dt = (now - last_timestamp).as_secs_f32();
            last_timestamp = now;

            let forward = vec3(yaw.cos(), -yaw.sin(), 0.0);
            let right = vec3(-yaw.sin(), -yaw.cos(), 0.0);
            let up = vec3(0.0, 0.0, 1.0);
            let delta_pos = if held_keys[&VirtualKeyCode::LShift] {
                10000.0
            } else {
                1000.0
            } * dt;
            if held_keys[&VirtualKeyCode::W] {
                pos += delta_pos * forward;
            }
            if held_keys[&VirtualKeyCode::S] {
                pos -= delta_pos * forward;
            }
            if held_keys[&VirtualKeyCode::A] {
                pos -= delta_pos * right;
            }
            if held_keys[&VirtualKeyCode::D] {
                pos += delta_pos * right;
            }
            if held_keys[&VirtualKeyCode::Space] {
                pos += delta_pos * up;
            }
            if held_keys[&VirtualKeyCode::LControl] {
                pos -= delta_pos * up;
            }

            let dimensions = display.get_framebuffer_dimensions();
            let proj = perspective(
                dimensions.0 as f32 / dimensions.1 as f32,
                radians(&vec1(90.0)).x,
                1.0,
                100000.0,
            );
            let view = look_at(
                &vec3(0.0, 0.0, 0.0),
                &vec3(1.0, 0.0, 0.0),
                &vec3(0.0, 0.0, 1.0),
            );
            let view = rotate(&view, pitch, &vec3(0.0, 1.0, 0.0));
            let view = rotate(&view, yaw, &vec3(0.0, 0.0, 1.0));
            let view = translate(&view, &-pos);
            let mvp_matrix = proj * view;

            let mut target = display.draw();
            target.clear_color_and_depth((0.5, 0.5, 0.5, 0.0), 1.0);
            target
                .draw(
                    &vertex_buffer,
                    &index_buffer,
                    &program,
                    &uniform! {
                        mvp_matrix: mvp_matrix.data.0,
                        lightmap: &lightmap_texture,
                    },
                    &DrawParameters {
                        depth: Depth {
                            test: DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling: BackfaceCullingMode::CullCounterClockwise,
                        ..Default::default()
                    },
                )
                .unwrap();
            target.finish().unwrap();

            let next_frame_time = Instant::now() + Duration::from_nanos(10_000_000);
            *control_flow = ControlFlow::WaitUntil(next_frame_time);
        }
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                if dragging {
                    yaw = (yaw + 0.01 * delta.0 as f32).rem_euclid(std::f32::consts::TAU);
                    pitch = (pitch - 0.01 * delta.1 as f32)
                        .clamp(radians(&vec1(-89.0)).x, radians(&vec1(89.0)).x)
                }
            }
            _ => (),
        },
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::MouseInput { button, state, .. } => {
                if button == MouseButton::Left {
                    dragging = state == ElementState::Pressed;
                    display
                        .gl_window()
                        .window()
                        .set_cursor_grab(dragging)
                        .unwrap();
                }
                if button == MouseButton::Right && state == ElementState::Pressed {
                    println!("pos: {:?}", pos);
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(code) = input.virtual_keycode {
                    if let Some(flag) = held_keys.get_mut(&code) {
                        *flag = input.state == ElementState::Pressed;
                    }
                }
            }
            _ => (),
        },
        _ => (),
    })
}

fn convert_vertex(
    bsp: Bsp,
    vertex_index: usize,
    tex_info: &bsp::TexInfo,
    face: &bsp::Face,
    lightmap_metadata: &LightmapMetadata,
    lightmap_image: &RgbU8Image,
) -> Vertex {
    let vertex = &bsp.vertices()[vertex_index];
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
    if patch_s < 0.0
        || patch_s > face.lightmap_texture_size_in_luxels[0] as f32
        || patch_t < 0.0
        || patch_t > face.lightmap_texture_size_in_luxels[1] as f32
    {
        panic!("lightmap coord out of range: ({}, {})", patch_s, patch_t);
    }
    let (patch_s, patch_t) = if lightmap_metadata.is_flipped {
        (patch_t, patch_s)
    } else {
        (patch_s, patch_t)
    };
    let s =
        (patch_s + lightmap_metadata.luxel_offset[0] as f32 + 0.5) / lightmap_image.width() as f32;
    let t =
        (patch_t + lightmap_metadata.luxel_offset[1] as f32 + 0.5) / lightmap_image.height() as f32;
    let vertex = Vertex {
        position: [vertex.x, vertex.y, vertex.z],
        lightmap_blend: 1.0,
        lightmap_coord: [s, t],
    };
    vertex
}

struct LightmapMetadata {
    luxel_offset: [usize; 2],
    is_flipped: bool,
}

fn build_lightmaps(bsp: Bsp) -> Result<(RgbU8Image, HashMap<i32, LightmapMetadata>)> {
    // Collect lightmap patches and insert them into a texture atlas.
    let mut lightmap_atlas = RgbU8TextureAtlas::new();
    let mut patch_ids_by_data_offset = HashMap::new();
    for (_, leaf) in bsp.enumerate_worldspawn_leaves() {
        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs != -1 && face.tex_info != -1 {
                // Import the lightmap patch if it hasn't already been imported.
                if !patch_ids_by_data_offset.contains_key(&face.light_ofs) {
                    // Allocate a patch in the lightmap texture atlas.
                    let width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                    let height = face.lightmap_texture_size_in_luxels[1] as usize + 1;

                    // Convert the luxel data.
                    // TODO: There can be multiple lightmap sets per face! Handle them!
                    let data = bsp
                        .lighting()
                        .at_offset(face.light_ofs, width * height)
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
    let (lightmap_data, offsets_by_patch_id) = lightmap_atlas.bake_smallest();
    let lightmap_metadata_by_data_offset = patch_ids_by_data_offset
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

    Ok((lightmap_data, lightmap_metadata_by_data_offset))
}
