#![deny(unsafe_op_in_unsafe_fn, unused_unsafe)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use glium::glutin::dpi::LogicalSize;
use glium::glutin::event::{DeviceEvent, Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::index::PrimitiveType;
use glium::program::ProgramCreationInput;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, SrgbFormat, SrgbTexture2d};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerWrapFunction};
use glium::{
    implement_vertex, uniform, BackfaceCullingMode, Depth, DepthTest, Display, DrawParameters,
    IndexBuffer, Program, Rect, Surface, VertexBuffer,
};
use memmap::Mmap;
use nalgebra_glm::{look_at, perspective, radians, rotate, translate, vec1, vec3};
use source_reader::asset::vmt::{LightmappedGeneric, Shader, VertexLitGeneric};
use source_reader::asset::AssetLoader;
use source_reader::bsp::{self, Bsp};
use source_reader::file::zip::ZipArchiveLoader;
use source_reader::file::{FallbackFileLoader, FileLoader};
use source_reader::geometry::convert_vertex;
use source_reader::lightmap::build_lightmaps;
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;
use texture_format::TextureFormat;

use crate::game_state::GameState;
use crate::texture::{
    create_texture, create_texture_encoded, AnyTexture2d, CreateCompressedSrgbTexture2dDxt1,
    CreateCompressedSrgbTexture2dDxt5, CreateSrgbTexture2dRgba8,
};

mod game_state;
mod texture;

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
    cluster_lightmap_textures: HashMap<i16, SrgbTexture2d>,
    vertices: Vec<Vertex>,
    indices_by_cluster_material: HashMap<i16, HashMap<VpkPath, Vec<u16>>>,
}

fn main() -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    let hl2_base = {
        let mut hl2_base = PathBuf::from(std::env::var("HOME").unwrap());
        hl2_base.extend([
            ".steam",
            "steam",
            "steamapps",
            "common",
            "Half-Life 2",
            "hl2",
        ]);
        hl2_base
    };
    #[cfg(target_os = "windows")]
    let hl2_base =
        PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Half-Life 2\\hl2");
    let hl2_misc = {
        let vpk_path = hl2_base.join("hl2_misc");
        Rc::new(Vpk::new(&vpk_path).with_context(|| format!("opening vpk {vpk_path:?}"))?)
    };

    let map_path = {
        let mut map_path = hl2_base.clone();
        map_path.push("maps");
        map_path.push("d1_trainstation_01.bsp");
        map_path
    };
    let bsp_file = File::open(map_path)?;
    let bsp_data = unsafe { Mmap::map(&bsp_file) }?;
    let bsp = Bsp::new(&bsp_data);
    let asset_loader = build_asset_loader(&hl2_base, bsp, Rc::clone(&hl2_misc))?;

    let events_loop = EventLoop::new();
    let display = Display::new(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024.0, 768.0))
            .with_title("bsp-loader-gl"),
        glium::glutin::ContextBuilder::new(),
        &events_loop,
    )
    .unwrap();

    let GraphicsData {
        cluster_lightmap_textures,
        vertices,
        indices_by_cluster_material,
    } = load_graphics_data(&display, bsp, &asset_loader)?;

    let program = build_shaders(&display)?;
    let vertex_buffer = VertexBuffer::new(&display, &vertices)?;
    let mut textures_by_path =
        load_textures(&display, &asset_loader, &indices_by_cluster_material)?;
    let batches_by_cluster = build_batches_by_cluster(
        &display,
        &asset_loader,
        indices_by_cluster_material,
        &textures_by_path,
    )?;

    // Begin model hack stuff.

    let mdl_path = VpkPath::new_with_prefix_and_extension("police", "models", "mdl");
    let mdl_data = match hl2_misc.load_file(&mdl_path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", mdl_path),
    };
    let mdl = source_reader::model::mdl::Mdl::new(&mdl_data);
    let vtx_path = VpkPath::new_with_prefix_and_extension("police", "models", "dx90.vtx");
    let vtx_data = match hl2_misc.load_file(&vtx_path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", vtx_path),
    };
    let vtx = source_reader::model::vtx::Vtx::new(&vtx_data);
    let vvd_path = VpkPath::new_with_prefix_and_extension("police", "models", "vvd");
    let vvd_data = match hl2_misc.load_file(&vvd_path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", vvd_path),
    };
    let vvd = source_reader::model::vvd::Vvd::new(&vvd_data);

    let model_program = build_model_shaders(&display)?;
    let (model_vertex_data, model_batches) =
        source_reader::model::glium::build_vertex_buffer(&display, &asset_loader, mdl, vtx, vvd);
    let model_vertex_buffer = VertexBuffer::new(&display, &model_vertex_data)?;
    load_model_textures(
        &display,
        &asset_loader,
        &mut textures_by_path,
        &model_batches,
    )?;
    let model_batches = model_batches
        .into_iter()
        .filter_map(ModelBatch::new)
        .collect::<Vec<_>>();

    // End model hack stuff.

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
                &batches_by_cluster,
                &program,
                &textures_by_path,
                &cluster_lightmap_textures,
                &model_vertex_buffer,
                &model_batches,
                &model_program,
            );

            let next_frame_time = Instant::now();
            *control_flow = ControlFlow::WaitUntil(next_frame_time);
        }
        _ => (),
    })
}

fn build_asset_loader<'a>(
    hl2_base: &Path,
    bsp: Bsp<'a>,
    hl2_misc: Rc<Vpk>,
) -> Result<AssetLoader<'a>> {
    let pak_loader = Rc::new(ZipArchiveLoader::new(bsp.pak_file()));
    let material_loader = Rc::new(FallbackFileLoader::new(vec![
        Rc::clone(&pak_loader) as Rc<dyn FileLoader>,
        hl2_misc,
    ]));
    let texture_loader = Rc::new(FallbackFileLoader::new(vec![
        Rc::clone(&pak_loader) as Rc<dyn FileLoader>,
        Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?),
    ]));
    Ok(AssetLoader::new(material_loader, texture_loader))
}

fn load_graphics_data(
    display: &Display,
    bsp: Bsp,
    asset_loader: &AssetLoader,
) -> Result<GraphicsData> {
    let (cluster_lightmaps, _displacement_lightmaps) = build_lightmaps(bsp)?;
    // TODO: Render displacements.

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct VertexKey {
        face: *const bsp::Face,
        vertex_index: usize,
    }

    let mut cluster_lightmap_texture_data: HashMap<i16, Vec<u8>> = HashMap::new();
    let mut vertices = Vec::new();
    let mut indices_by_cluster_material: HashMap<i16, HashMap<VpkPath, Vec<u16>>> = HashMap::new();
    for leaf in bsp.iter_worldspawn_leaves() {
        let cluster = leaf.cluster();
        if cluster == -1 {
            // Leaf is not potentially visible from anywhere.
            continue;
        }
        let cluster_lightmap = match cluster_lightmaps.get(&cluster) {
            Some(lightmap) => lightmap,
            // TODO: Render non-lightmapped geometry.
            None => continue,
        };
        let lightmap_texture_data = cluster_lightmap_texture_data
            .entry(cluster)
            .or_insert_with(|| vec![0u8; 3 * cluster_lightmap.width * cluster_lightmap.height]);
        let indices_by_material = indices_by_cluster_material.entry(cluster).or_default();
        let mut emitted_vertices_by_source = HashMap::new();

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

            // Write texels to the lightmap.
            {
                let patch_width = face.lightmap_texture_size_in_luxels[0] as usize + 1;
                let patch_height = face.lightmap_texture_size_in_luxels[1] as usize + 1;
                assert_eq!(lightmap_metadata.luxel_offset[0] % 4, 0);
                assert_eq!(lightmap_metadata.luxel_offset[1] % 4, 0);

                let mut src_offset = face.light_ofs as usize;
                for src_dy in 0..patch_height {
                    for src_dx in 0..patch_width {
                        let rgb = bsp.lighting().at_offset(src_offset, 1)[0].to_srgb8();
                        src_offset += 4;

                        let (dst_x, dst_y) = if lightmap_metadata.is_flipped {
                            (
                                lightmap_metadata.luxel_offset[0] + src_dy,
                                lightmap_metadata.luxel_offset[1] + src_dx,
                            )
                        } else {
                            (
                                lightmap_metadata.luxel_offset[0] + src_dx,
                                lightmap_metadata.luxel_offset[1] + src_dy,
                            )
                        };
                        let dst_offset = 3 * (cluster_lightmap.width * dst_y + dst_x);
                        lightmap_texture_data[dst_offset..dst_offset + 3].copy_from_slice(&rgb);
                    }
                }
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
                    let vertex =
                        convert_vertex(bsp, Some(cluster_lightmap), face, tex_info, vertex_index);

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

    let cluster_lightmap_textures: HashMap<i16, SrgbTexture2d> = cluster_lightmap_texture_data
        .into_iter()
        .map(|(cluster_index, lightmap_texture_data)| {
            let cluster_lightmap = &cluster_lightmaps[&cluster_index];

            let lightmap_texture = SrgbTexture2d::empty_with_format(
                display,
                SrgbFormat::U8U8U8,
                MipmapsOption::NoMipmap,
                cluster_lightmap.width as u32,
                cluster_lightmap.height as u32,
            )?;
            lightmap_texture.write(
                Rect {
                    left: 0,
                    bottom: 0,
                    width: cluster_lightmap.width as u32,
                    height: cluster_lightmap.height as u32,
                },
                RawImage2d {
                    data: Cow::Owned(lightmap_texture_data),
                    width: cluster_lightmap.width as u32,
                    height: cluster_lightmap.height as u32,
                    format: ClientFormat::U8U8U8,
                },
            );
            Ok((cluster_index, lightmap_texture))
        })
        .collect::<Result<_>>()?;

    Ok(GraphicsData {
        cluster_lightmap_textures,
        vertices,
        indices_by_cluster_material,
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
            rendered_color = lightmap_color * base_color * 4.59479;
        }
    "#;
    Ok(Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: VERTEX_SHADER_SOURCE,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: FRAGMENT_SHADER_SOURCE,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
        },
    )?)
}

fn build_model_shaders(display: &Display) -> Result<Program> {
    const VERTEX_SHADER_SOURCE: &str = r#"
        #version 330

        uniform mat4 mvp_matrix;

        in vec3 position;
        in vec3 normal;
        in vec2 tex_coord;

        out vec3 interpolated_normal;
        out vec2 interpolated_tex_coord;

        void main() {
            gl_Position = mvp_matrix * vec4(position, 1.0);
            interpolated_normal = normal;
            interpolated_tex_coord = tex_coord;
        }
    "#;
    const FRAGMENT_SHADER_SOURCE: &str = r#"
        #version 330

        uniform sampler2D base_map;

        in vec3 interpolated_normal;
        in vec2 interpolated_tex_coord;

        out vec4 rendered_color;

        void main() {
            rendered_color = vec4(
                texture(base_map, interpolated_tex_coord).rgb,
                1.0);
        }
    "#;
    Ok(Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: VERTEX_SHADER_SOURCE,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: FRAGMENT_SHADER_SOURCE,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
        },
    )?)
}

fn load_textures(
    display: &Display,
    asset_loader: &AssetLoader,
    indices_by_cluster_material: &HashMap<i16, HashMap<VpkPath, Vec<u16>>>,
) -> Result<HashMap<VpkPath, AnyTexture2d>> {
    let mut textures_by_path = HashMap::new();
    for (_cluster_index, indices_by_material) in indices_by_cluster_material {
        for material_path in indices_by_material.keys() {
            let material = asset_loader.get_material(material_path)?;
            if let Shader::LightmappedGeneric(LightmappedGeneric {
                base_texture_path, ..
            }) = material.shader()
            {
                if !textures_by_path.contains_key(base_texture_path) {
                    load_texture(
                        display,
                        asset_loader,
                        &mut textures_by_path,
                        base_texture_path,
                    )?;
                }
            }
        }
    }
    Ok(textures_by_path)
}

fn load_texture(
    display: &Display,
    asset_loader: &AssetLoader,
    textures_by_path: &mut HashMap<VpkPath, AnyTexture2d>,
    texture_path: &VpkPath,
) -> Result<()> {
    let base_texture = asset_loader.get_texture(texture_path)?;
    let gl_texture = match base_texture.format() {
        TextureFormat::Bgr8 => create_texture_encoded::<CreateSrgbTexture2dRgba8>(
            display,
            &base_texture,
            TextureFormat::Rgb8,
        )?,
        TextureFormat::Dxt1 => {
            create_texture::<CreateCompressedSrgbTexture2dDxt1>(display, &base_texture)?
        }
        TextureFormat::Dxt5 => {
            create_texture::<CreateCompressedSrgbTexture2dDxt5>(display, &base_texture)?
        }
        format => panic!("unexpected texture format: {:?}", format),
    };
    textures_by_path.insert(texture_path.to_owned(), gl_texture);
    Ok(())
}

fn load_model_textures(
    display: &Display,
    asset_loader: &AssetLoader,
    textures_by_path: &mut HashMap<VpkPath, AnyTexture2d>,
    batches: &[source_reader::model::glium::Batch],
) -> Result<()> {
    for batch in batches {
        let base_map = match batch.base_map.as_ref() {
            Some(base_map) => base_map,
            None => continue,
        };
        match base_map.shader() {
            Shader::VertexLitGeneric(VertexLitGeneric {
                base_texture_path, ..
            }) => {
                if !textures_by_path.contains_key(base_texture_path) {
                    load_texture(display, asset_loader, textures_by_path, base_texture_path)?;
                }
            }
            shader => panic!("unexpected model shader: {:?}", shader),
        }
    }
    Ok(())
}

struct Batch {
    index_buffer: IndexBuffer<u16>,
    base_map_path: VpkPath,
    inv_base_map_size: [f32; 2],
}

fn build_batches_by_cluster(
    display: &Display,
    asset_loader: &AssetLoader,
    indices_by_cluster_material: HashMap<i16, HashMap<VpkPath, Vec<u16>>>,
    textures_by_path: &HashMap<VpkPath, AnyTexture2d>,
) -> Result<HashMap<i16, Vec<Batch>>> {
    let mut batches_by_cluster = HashMap::new();
    for (cluster_index, indices_by_material) in indices_by_cluster_material {
        let mut batches = Vec::new();
        for (material_path, indices) in indices_by_material {
            let material = asset_loader.get_material(&material_path)?;
            if let Shader::LightmappedGeneric(LightmappedGeneric {
                base_texture_path, ..
            }) = material.shader()
            {
                let index_buffer =
                    IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices)?;
                if let Some(base_map_texture) = textures_by_path.get(base_texture_path) {
                    batches.push(Batch {
                        index_buffer,
                        base_map_path: base_texture_path.to_owned(),
                        inv_base_map_size: [
                            1.0 / base_map_texture.width() as f32,
                            1.0 / base_map_texture.height() as f32,
                        ],
                    });
                }
            }
        }
        batches_by_cluster.insert(cluster_index, batches);
    }
    Ok(batches_by_cluster)
}

struct ModelBatch {
    index_buffer: IndexBuffer<u16>,
    base_map_path: VpkPath,
}

impl ModelBatch {
    fn new(batch: source_reader::model::glium::Batch) -> Option<Self> {
        match batch.base_map?.shader() {
            Shader::VertexLitGeneric(VertexLitGeneric {
                base_texture_path, ..
            }) => Some(ModelBatch {
                index_buffer: batch.index_buffer,
                base_map_path: base_texture_path.to_owned(),
            }),
            shader => panic!("unexpected model shader: {:?}", shader),
        }
    }
}

fn draw(
    display: &Display,
    game_state: &GameState,
    vertex_buffer: &VertexBuffer<Vertex>,
    batches_by_cluster: &HashMap<i16, Vec<Batch>>,
    program: &Program,
    textures_by_path: &HashMap<VpkPath, AnyTexture2d>,
    cluster_lightmap_textures: &HashMap<i16, SrgbTexture2d>,
    model_vertex_buffer: &VertexBuffer<source_reader::model::glium::Vertex>,
    model_batches: &[ModelBatch],
    model_program: &Program,
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
    target.clear_color_and_depth((0.5, 0.5, 0.5, 1.0), 1.0);
    for (cluster_index, batches) in batches_by_cluster {
        for batch in batches {
            let base_texture = &textures_by_path[&batch.base_map_path];
            match base_texture {
                AnyTexture2d::Texture2d(x) => target
                    .draw(
                        vertex_buffer,
                        &batch.index_buffer,
                        &program,
                        &uniform! {
                            mvp_matrix: mvp_matrix.data.0,
                            lightmap: Sampler::new(&cluster_lightmap_textures[cluster_index])
                                .wrap_function(SamplerWrapFunction::Clamp)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::Nearest),
                            base_map: Sampler::new(x)
                                .wrap_function(SamplerWrapFunction::Repeat)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::LinearMipmapNearest)
                                .anisotropy(16),
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
                    .unwrap(),
                AnyTexture2d::SrgbTexture2d(x) => target
                    .draw(
                        vertex_buffer,
                        &batch.index_buffer,
                        &program,
                        &uniform! {
                            mvp_matrix: mvp_matrix.data.0,
                            lightmap: Sampler::new(&cluster_lightmap_textures[cluster_index])
                                .wrap_function(SamplerWrapFunction::Clamp)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::Nearest),
                            base_map: Sampler::new(x)
                                .wrap_function(SamplerWrapFunction::Repeat)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                                .anisotropy(16),
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
                    .unwrap(),
                AnyTexture2d::CompressedTexture2d(x) => target
                    .draw(
                        vertex_buffer,
                        &batch.index_buffer,
                        &program,
                        &uniform! {
                            mvp_matrix: mvp_matrix.data.0,
                            lightmap: Sampler::new(&cluster_lightmap_textures[cluster_index])
                                .wrap_function(SamplerWrapFunction::Clamp)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::Nearest),
                            base_map: Sampler::new(x)
                                .wrap_function(SamplerWrapFunction::Repeat)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                                .anisotropy(16),
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
                    .unwrap(),
                AnyTexture2d::CompressedSrgbTexture2d(x) => target
                    .draw(
                        vertex_buffer,
                        &batch.index_buffer,
                        &program,
                        &uniform! {
                            mvp_matrix: mvp_matrix.data.0,
                            lightmap: Sampler::new(&cluster_lightmap_textures[cluster_index])
                                .wrap_function(SamplerWrapFunction::Clamp)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::Nearest),
                            base_map: Sampler::new(x)
                                .wrap_function(SamplerWrapFunction::Repeat)
                                .magnify_filter(MagnifySamplerFilter::Linear)
                                .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                                .anisotropy(16),
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
                    .unwrap(),
            }
        }
    }

    for batch in model_batches {
        let base_texture = &textures_by_path[&batch.base_map_path];
        match base_texture {
            AnyTexture2d::Texture2d(x) => target
                .draw(
                    model_vertex_buffer,
                    &batch.index_buffer,
                    &model_program,
                    &uniform! {
                        mvp_matrix: mvp_matrix.data.0,
                        base_map: Sampler::new(x)
                            .wrap_function(SamplerWrapFunction::Repeat)
                            .magnify_filter(MagnifySamplerFilter::Linear)
                            .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                            .anisotropy(16),
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
                .unwrap(),
            AnyTexture2d::SrgbTexture2d(x) => target
                .draw(
                    model_vertex_buffer,
                    &batch.index_buffer,
                    &model_program,
                    &uniform! {
                        mvp_matrix: mvp_matrix.data.0,
                        base_map: Sampler::new(x)
                            .wrap_function(SamplerWrapFunction::Repeat)
                            .magnify_filter(MagnifySamplerFilter::Linear)
                            .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                            .anisotropy(16),
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
                .unwrap(),
            AnyTexture2d::CompressedTexture2d(x) => target
                .draw(
                    model_vertex_buffer,
                    &batch.index_buffer,
                    &model_program,
                    &uniform! {
                        mvp_matrix: mvp_matrix.data.0,
                        base_map: Sampler::new(x)
                            .wrap_function(SamplerWrapFunction::Repeat)
                            .magnify_filter(MagnifySamplerFilter::Linear)
                            .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                            .anisotropy(16),
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
                .unwrap(),
            AnyTexture2d::CompressedSrgbTexture2d(x) => target
                .draw(
                    model_vertex_buffer,
                    &batch.index_buffer,
                    &model_program,
                    &uniform! {
                        mvp_matrix: mvp_matrix.data.0,
                        base_map: Sampler::new(x)
                            .wrap_function(SamplerWrapFunction::Repeat)
                            .magnify_filter(MagnifySamplerFilter::Linear)
                            .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
                            .anisotropy(16),
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
                .unwrap(),
        }
    }

    target.finish().unwrap();
}
