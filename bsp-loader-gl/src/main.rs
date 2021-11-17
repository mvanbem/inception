#![deny(unsafe_op_in_unsafe_fn, unused_unsafe)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
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

use crate::bsp::Bsp;
use crate::texture_atlas::{RgbU8Image, RgbU8TextureAtlas};

mod bsp;
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

fn main() -> Result<()> {
    let bsp_data = std::fs::read("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Half-Life 2\\hl2\\maps\\d1_trainstation_01.bsp")?;
    let bsp = Bsp::new(&bsp_data);

    let (lightmap_image, lightmap_metadata_by_data_offset) = build_lightmaps(bsp)?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut emitted_vertices_by_source = HashMap::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        for face in bsp.iter_faces_from_leaf(leaf) {
            if face.light_ofs == -1 || face.tex_info == -1 {
                // Not a textured lightmapped surface.
                continue;
            }
            let lightmap_metadata = &lightmap_metadata_by_data_offset[&face.light_ofs];
            let tex_info = &bsp.tex_infos()[face.tex_info as usize];

            let mut first_index = None;
            let mut prev_index = None;
            for vertex_index in bsp.iter_vertex_indices_from_face(face) {
                let key = VertexKey { face, vertex_index };
                let remapped_index = if emitted_vertices_by_source.contains_key(&key) {
                    *emitted_vertices_by_source.get(&key).unwrap()
                } else {
                    let vertex = &bsp.vertices()[vertex_index];

                    // Recover lightmap patch texture coordinates.
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

                    // Flip patch coordinates if the patch is flipped in the texture atlas.
                    let (patch_s, patch_t) = if lightmap_metadata.is_flipped {
                        (patch_t, patch_s)
                    } else {
                        (patch_s, patch_t)
                    };

                    // Translate lightmap patch texture coordinates to lightmap atlas texture
                    // coordinates.
                    let s = (patch_s + lightmap_metadata.luxel_offset[0] as f32 + 0.5)
                        / lightmap_image.width() as f32;
                    let t = (patch_t + lightmap_metadata.luxel_offset[1] as f32 + 0.5)
                        / lightmap_image.height() as f32;

                    // Emit the vertex.
                    let remapped_index = u16::try_from(vertices.len()).unwrap();
                    vertices.push(Vertex {
                        position: [vertex.x, vertex.y, vertex.z],
                        lightmap_blend: 1.0,
                        lightmap_coord: [s, t],
                    });
                    emitted_vertices_by_source.insert(key, remapped_index);
                    remapped_index
                };

                if first_index.is_none() {
                    first_index = Some(remapped_index);
                }
                if let Some(prev_index) = prev_index {
                    indices.push(first_index.unwrap());
                    indices.push(prev_index);
                    indices.push(remapped_index);
                }
                prev_index = Some(remapped_index);
            }
        }
    }

    if true {
        lightmap_image.write_to_png("lightmap_atlas.png")?;
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
    let mut yaw: f32 = 0.0;
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

struct LightmapMetadata {
    luxel_offset: [usize; 2],
    is_flipped: bool,
}

fn build_lightmaps(bsp: Bsp) -> Result<(RgbU8Image, HashMap<i32, LightmapMetadata>)> {
    // Collect lightmap patches and insert them into a texture atlas.
    let mut lightmap_atlas = RgbU8TextureAtlas::new();
    let mut patch_ids_by_data_offset = HashMap::new();
    for leaf in bsp.iter_worldspawn_leaves() {
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
