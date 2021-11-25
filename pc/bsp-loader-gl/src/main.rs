#![deny(unsafe_op_in_unsafe_fn, unused_unsafe)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use anyhow::Result;
use glium::glutin::dpi::LogicalSize;
use glium::glutin::event::{DeviceEvent, Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::index::PrimitiveType;
use glium::texture::compressed_srgb_texture2d::CompressedSrgbTexture2d;
use glium::texture::{ClientFormat, CompressedMipmapsOption, CompressedSrgbFormat, RawImage2d};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler};
use glium::{
    implement_vertex, uniform, BackfaceCullingMode, Depth, DepthTest, Display, DrawParameters,
    IndexBuffer, Program, Rect, Surface, VertexBuffer,
};
use memmap::Mmap;
use nalgebra_glm::{look_at, perspective, radians, rotate, translate, vec1, vec3};
use source_reader::asset::vmt::Shader;
use source_reader::asset::vtf::{ImageData, ImageFormat};
use source_reader::asset::AssetLoader;
use source_reader::bsp::{self, Bsp};
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::{FallbackFileLoader, FileLoader};
use source_reader::geometry::convert_vertex;
use source_reader::lightmap::{build_lightmaps, Lightmap};
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;

use crate::game_state::GameState;

mod game_state;

#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    lightmap_coord: [f32; 2],
    texture_coord: [f32; 2],
}

impl From<source_reader::geometry::Vertex> for Vertex {
    fn from(v: source_reader::geometry::Vertex) -> Self {
        Self {
            position: v.position,
            lightmap_coord: v.lightmap_coord,
            texture_coord: v.texture_coord,
        }
    }
}

implement_vertex!(Vertex, position, lightmap_coord, texture_coord);

struct GraphicsData {
    lightmap: Lightmap,
    vertices: Vec<Vertex>,
    indices_by_material: HashMap<VpkPath, Vec<u16>>,
}

fn main() -> Result<()> {
    let hl2_base: &Path =
        Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Half-Life 2\\hl2");
    let bsp_file = File::open(hl2_base.join("maps\\d1_trainstation_01.bsp"))?;
    let bsp_data = unsafe { Mmap::map(&bsp_file) }?;
    let bsp = Bsp::new(&bsp_data);
    let asset_loader = build_asset_loader(hl2_base, bsp)?;

    let GraphicsData {
        lightmap,
        vertices,
        indices_by_material,
    } = load_graphics_data(bsp, &asset_loader)?;

    let events_loop = EventLoop::new();
    let display = Display::new(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024.0, 768.0))
            .with_title("bsp-loader-gl"),
        glium::glutin::ContextBuilder::new().with_double_buffer(Some(true)),
        &events_loop,
    )
    .unwrap();

    let program = build_shaders(&display)?;
    let lightmap_texture = build_lightmap_texture(&display, &lightmap)?;
    let vertex_buffer = VertexBuffer::new(&display, &vertices)?;
    let textures_by_path = load_textures(&display, &asset_loader, &indices_by_material)?;
    let batches = build_batches(
        &display,
        asset_loader,
        indices_by_material,
        &textures_by_path,
    )?;

    let mut game_state = GameState::new();
    events_loop.run(move |event, _target, control_flow| match event {
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                game_state.handle_mouse_motion(delta);
            }
            _ => (),
        },
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::MouseInput { button, state, .. } => {
                game_state.handle_mouse_input(&display, button, state);
            }
            WindowEvent::KeyboardInput { input, .. } => {
                game_state.handle_keyboard_input(input);
            }
            _ => (),
        },
        Event::MainEventsCleared => {
            game_state.step();

            draw(
                &display,
                &game_state,
                &vertex_buffer,
                &batches,
                &program,
                &textures_by_path,
                &lightmap_texture,
            );

            let next_frame_time = Instant::now();
            *control_flow = ControlFlow::WaitUntil(next_frame_time);
        }
        _ => (),
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

fn load_graphics_data(bsp: Bsp, asset_loader: &AssetLoader) -> Result<GraphicsData> {
    let lightmap = build_lightmaps(bsp)?;

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct VertexKey {
        face: *const bsp::Face,
        vertex_index: usize,
    }

    let mut vertices = Vec::new();
    let mut indices_by_material: HashMap<VpkPath, Vec<u16>> = HashMap::new();
    let mut emitted_vertices_by_source = HashMap::new();
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
            let _material = asset_loader.get_material(&material_path)?;
            let indices = indices_by_material
                .entry(material_path.clone())
                .or_default();

            let mut first_index = None;
            let mut prev_index = None;
            for vertex_index in bsp.iter_vertex_indices_from_face(face) {
                let key = VertexKey { face, vertex_index };
                let remapped_index = if emitted_vertices_by_source.contains_key(&key) {
                    *emitted_vertices_by_source.get(&key).unwrap()
                } else {
                    let vertex = convert_vertex(
                        bsp,
                        &lightmap.image,
                        lightmap_metadata,
                        face,
                        tex_info,
                        vertex_index,
                    );

                    // Emit the vertex.
                    let remapped_index = u16::try_from(vertices.len()).unwrap();
                    vertices.push(Vertex::from(vertex));
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

    Ok(GraphicsData {
        lightmap,
        vertices,
        indices_by_material,
    })
}

fn build_shaders(display: &Display) -> Result<Program> {
    const VERTEX_SHADER_SOURCE: &str = r#"
        #version 330

        uniform mat4 mvp_matrix;
        uniform vec2 inv_base_map_size;

        in vec3 position;
        in vec2 lightmap_coord;
        in vec2 texture_coord;

        out vec2 interpolated_lightmap_coord;
        out vec2 interpolated_texture_coord;

        void main() {
            gl_Position = mvp_matrix * vec4(position, 1.0);
            interpolated_lightmap_coord = lightmap_coord;
            interpolated_texture_coord = texture_coord * inv_base_map_size;
        }
    "#;
    const FRAGMENT_SHADER_SOURCE: &str = r#"
        #version 330

        uniform sampler2D lightmap;
        uniform sampler2D base_map;

        in vec2 interpolated_lightmap_coord;
        in vec2 interpolated_texture_coord;

        out vec4 rendered_color;

        void main() {
            vec4 lightmap_color = vec4(texture(lightmap, interpolated_lightmap_coord).rgb, 1.0);
            vec4 base_color = texture(base_map, interpolated_texture_coord);
            rendered_color = lightmap_color * base_color * 2.0;
        }
    "#;
    Ok(Program::from_source(
        display,
        VERTEX_SHADER_SOURCE,
        FRAGMENT_SHADER_SOURCE,
        None,
    )?)
}

fn build_lightmap_texture(
    display: &Display,
    lightmap: &Lightmap,
) -> Result<CompressedSrgbTexture2d> {
    Ok(CompressedSrgbTexture2d::with_format(
        display,
        RawImage2d {
            data: Cow::Borrowed(lightmap.image.data()),
            width: lightmap.image.width() as u32,
            height: lightmap.image.height() as u32,
            format: ClientFormat::U8U8U8,
        },
        CompressedSrgbFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
    )?)
}

fn load_textures(
    display: &Display,
    asset_loader: &AssetLoader,
    indices_by_material: &HashMap<VpkPath, Vec<u16>>,
) -> Result<HashMap<VpkPath, CompressedSrgbTexture2d>> {
    let mut textures_by_path = HashMap::new();
    let mut total_texture_size = 0;
    for material_path in indices_by_material.keys() {
        let material = asset_loader.get_material(material_path)?;
        if let Shader::LightmappedGeneric { base_texture, .. } = material.shader() {
            if !textures_by_path.contains_key(base_texture.path()) {
                // Load this texture.
                let (base_map_width, base_map_height, mips) = match material.shader() {
                    Shader::LightmappedGeneric { base_texture, .. } => match base_texture.data() {
                        Some(ImageData {
                            format: ImageFormat::Dxt1,
                            mips,
                        }) => (base_texture.width(), base_texture.height(), mips),
                        None => continue,
                    },
                    _ => continue,
                };

                let base_map_texture = CompressedSrgbTexture2d::empty_with_format(
                    display,
                    CompressedSrgbFormat::S3tcDxt1NoAlpha,
                    CompressedMipmapsOption::EmptyMipmapsMax(mips.len() as u32 - 1),
                    base_map_width,
                    base_map_height,
                )?;
                for (mip_level, mip_data) in mips.iter().enumerate() {
                    let mip_texture = base_map_texture.mipmap(mip_level as u32).unwrap();
                    let mip_width = mip_texture.width();
                    let mip_height = mip_texture.height();
                    assert_eq!(
                        mip_width.max(4) as usize * mip_height.max(4) as usize / 2,
                        mip_data.len(),
                    );
                    total_texture_size += mip_data.len();
                    mip_texture
                        .write_compressed_data(
                            Rect {
                                left: 0,
                                bottom: 0,
                                width: mip_width,
                                height: mip_height,
                            },
                            mip_data,
                            mip_width,
                            mip_height,
                            CompressedSrgbFormat::S3tcDxt1NoAlpha,
                        )
                        .unwrap();
                }
                textures_by_path.insert(base_texture.path().to_owned(), base_map_texture);
            }
        }
    }
    println!("Total texture size: {}", total_texture_size);
    Ok(textures_by_path)
}

struct Batch {
    index_buffer: IndexBuffer<u16>,
    base_map_path: VpkPath,
    inv_base_map_size: [f32; 2],
}

fn build_batches(
    display: &Display,
    asset_loader: AssetLoader,
    indices_by_material: HashMap<VpkPath, Vec<u16>>,
    textures_by_path: &HashMap<VpkPath, CompressedSrgbTexture2d>,
) -> Result<Vec<Batch>> {
    let mut batches = Vec::new();
    for (material_path, indices) in indices_by_material {
        let material = asset_loader.get_material(&material_path)?;
        if let Shader::LightmappedGeneric { base_texture, .. } = material.shader() {
            let index_buffer = IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices)?;
            if let Some(base_map_texture) = textures_by_path.get(base_texture.path()) {
                batches.push(Batch {
                    index_buffer,
                    base_map_path: base_texture.path().to_owned(),
                    inv_base_map_size: [
                        1.0 / base_map_texture.width() as f32,
                        1.0 / base_map_texture.height() as f32,
                    ],
                });
            }
        }
    }
    Ok(batches)
}

fn draw(
    display: &Display,
    game_state: &GameState,
    vertex_buffer: &VertexBuffer<Vertex>,
    batches: &[Batch],
    program: &Program,
    textures_by_path: &HashMap<VpkPath, CompressedSrgbTexture2d>,
    lightmap_texture: &CompressedSrgbTexture2d,
) {
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
    let view = rotate(&view, game_state.pitch, &vec3(0.0, 1.0, 0.0));
    let view = rotate(&view, game_state.yaw, &vec3(0.0, 0.0, 1.0));
    let view = translate(&view, &-game_state.pos);
    let mvp_matrix = proj * view;

    let mut target = display.draw();
    target.clear_color_and_depth((0.5, 0.5, 0.5, 0.0), 1.0);
    for batch in batches {
        target
            .draw(
                vertex_buffer,
                &batch.index_buffer,
                &program,
                &uniform! {
                    mvp_matrix: mvp_matrix.data.0,
                    lightmap: lightmap_texture,
                    base_map: Sampler::new(&textures_by_path[&batch.base_map_path])
                        .magnify_filter(MagnifySamplerFilter::Linear)
                        .minify_filter(MinifySamplerFilter::LinearMipmapLinear),
                    inv_base_map_size: batch.inv_base_map_size,
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
    }
    target.finish().unwrap();
}
