#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(allocator_api)]
#![feature(core_intrinsics)]
#![feature(start)]

extern crate alloc;
#[cfg(test)]
extern crate std;
#[macro_use(include_bytes_align_as)]
extern crate include_bytes_align_as;

use core::ffi::c_void;
use core::mem::zeroed;
use core::ops::Deref;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use aligned::A32;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use derive_try_from_primitive::TryFromPrimitive;
use font_gx::TextRenderer;
use gamecube_mmio::dvd_interface::DvdInterface;
use gamecube_mmio::processor_interface::ProcessorInterface;
use gamecube_shader::FLAT_TEXTURED_SHADER;
use inception_render_common::bytecode::{BytecodeOp, BytecodeReader};
use inception_render_common::map_data::{MapData, TextureTableEntry};
use num_traits::float::FloatCore;
use ogc_sys::*;

use crate::lightmap::Lightmap;
use crate::loader::Loader;
use crate::shaders::flat_vertex_color::FLAT_VERTEX_COLOR_SHADER;
use crate::shaders::lightmapped::LIGHTMAPPED_SHADER;
use crate::shaders::lightmapped_baaa::LIGHTMAPPED_BAAA_SHADER;
use crate::shaders::self_illum::SELF_ILLUM_SHADER;
use crate::shaders::unlit_generic::UNLIT_GENERIC_SHADER;
use crate::shaders::world_vertex_transition::WORLD_VERTEX_TRANSITION_SHADER;
use crate::visibility::{ClusterIndex, Visibility};

mod iso9660;
mod lightmap;
mod loader;
mod net;
mod shaders;
mod visibility;

static UI_FONT: &[u8] = include_bytes_align_as!(A32, "../../../build/ui_font.dat");

static XFB_FRONT: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static XFB_BACK: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static GP_FIFO: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static DO_COPY: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicUsize = AtomicUsize::new(0);
static LAST_FRAME_FRAMES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "wii")]
fn get_widescreen_setting() -> bool {
    unsafe { CONF_GetAspectRatio() != 0 }
}

#[cfg(not(feature = "wii"))]
fn get_widescreen_setting() -> bool {
    true // Probably a bad default, but that's what my test setup wants.
}

fn configure_loader() -> impl Loader {
    #[cfg(feature = "dvd_loader")]
    {
        return crate::loader::dvd_gcm_loader::DvdGcmLoader::new((
            gamecube_dvd_driver::DvdDriver::new(DvdInterface::new()),
            ProcessorInterface::new(),
        ));
    }

    #[cfg(feature = "ftp_loader")]
    {
        return crate::loader::ftp_loader::FtpLoader::new(crate::net::SocketAddr::new(
            [10, 0, 1, 104],
            21,
        ));
    }

    #[cfg(feature = "embedded_loader")]
    {
        return crate::loader::embedded_loader::EmbeddedLoader::new(());
    }

    #[allow(unreachable_code)]
    {
        unreachable!()
    }
}

fn select_map(loader: &mut impl Loader) -> String {
    unsafe {
        loop {
            libc::printf(b"\x1b[2JFetching map list...\n\0".as_ptr());
            let mut maps = loader.maps();

            if maps.is_empty() {
                libc::printf(b"Map list was empty!\0".as_ptr());
                loop {}
            } else if maps.len() == 1 {
                return maps.swap_remove(0);
            }

            libc::printf(
                b"\nSelect a map:\n\n\x1b[s\n\n\
            D-Pad: Select  \x1a: +1  \x1b: -1  \x18: +10  \x19: -10\n\
            B:     Refresh\n\
            A:     Confirm\n\
            Start: Return to loader\0"
                    .as_ptr(),
            );
            let mut index = 0usize;
            'select: loop {
                let buf = format!(
                    "\x1b[u\x1b[K    ({}/{}) {}\n\0",
                    index + 1,
                    maps.len(),
                    maps[index],
                );
                libc::printf(b"%s\0".as_ptr(), buf.as_ptr());

                loop {
                    VIDEO_WaitVSync();
                    PAD_ScanPads();
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_START as u16) != 0 {
                        libc::exit(0);
                    }
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_UP as u16) != 0 {
                        if index < maps.len() - 1 {
                            index = (index + 10).min(maps.len() - 1);
                        } else {
                            index = 0;
                        }
                        break;
                    }
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_DOWN as u16) != 0 {
                        if index > 0 {
                            index = index.saturating_sub(10);
                        } else {
                            index = maps.len() - 1;
                        }
                        break;
                    }
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_LEFT as u16) != 0 {
                        if index > 0 {
                            index -= 1;
                        } else {
                            index = maps.len() - 1;
                        }
                        break;
                    }
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_RIGHT as u16) != 0 {
                        if index < maps.len() - 1 {
                            index += 1;
                        } else {
                            index = 0;
                        }
                        break;
                    }
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_A as u16) != 0 {
                        return maps.swap_remove(index);
                    }
                    if (PAD_ButtonsDown(0) & PAD_BUTTON_B as u16) != 0 {
                        break 'select;
                    }
                }
            }
        }
    }
}

/// # SAFETY
///
/// This function writes to memory that aliases `map_data` and so is fundamentally unsound. That
/// said, the memory in question is a GX display list that the CPU will never read. Let's hope the
/// optimizer doesn't strip out these writes.
unsafe fn relocate_references<Data: Deref<Target = [u8]>>(map_data: &MapData<Data>) {
    unsafe {
        for entry in map_data.cluster_geometry_references() {
            let display_list_ptr: *mut u32 = map_data
                .cluster_geometry_display_lists()
                .as_ptr()
                .cast_mut()
                .offset(entry.display_list_offset as isize)
                .cast();
            let image_ptr = map_data
                .texture_data()
                .as_ptr()
                .offset(map_data.texture_table()[entry.texture_id as usize].start_offset as isize);
            let image_reg_value = ((image_ptr as u32) >> 5) & 0x00ffffff;
            display_list_ptr.write(display_list_ptr.read() & 0xff000000 | image_reg_value);
        }
        DCFlushRange(
            map_data.cluster_geometry_display_lists().as_ptr() as _,
            map_data.cluster_geometry_display_lists().len() as u32,
        );

        for entry in map_data.displacement_references() {
            let display_list_ptr: *mut u32 = map_data
                .displacement_display_lists()
                .as_ptr()
                .cast_mut()
                .offset(entry.display_list_offset as isize)
                .cast();
            let image_ptr = map_data
                .texture_data()
                .as_ptr()
                .offset(map_data.texture_table()[entry.texture_id as usize].start_offset as isize);
            let image_reg_value = ((image_ptr as u32) >> 5) & 0x00ffffff;
            display_list_ptr.write(display_list_ptr.read() & 0xff000000 | image_reg_value);
        }
        DCFlushRange(
            map_data.displacement_display_lists().as_ptr() as _,
            map_data.displacement_display_lists().len() as u32,
        );
    }
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        init_for_console();

        let mut loader = configure_loader();

        loop {
            PENDING_GAME_STATE_CHANGE.store(GameStateChange::None as u32, Ordering::SeqCst);

            let (rmode, width, height) = init_for_console();

            // Compute logical height.
            let height = if (*rmode).aa != 0 { 2 * height } else { height };

            let map = select_map(&mut loader);
            libc::printf(b"Loading map...\n\0".as_ptr());
            let map_data = loader.load_map(&map);

            relocate_references(&map_data);

            init_for_3d(&*rmode);

            // Set up texture objects for cluster lightmaps.
            let mut cluster_lightmaps = Vec::new();
            for entry in map_data.lightmap_cluster_table() {
                let mut lightmap = Lightmap::new(&entry.common);
                lightmap.update(&map_data, &entry.common, 0);
                cluster_lightmaps.push(lightmap);
            }
            let mut displacement_lightmaps = BTreeMap::new();
            for entry in map_data.lightmap_displacement_table() {
                let mut lightmap = Lightmap::new(&entry.common);
                lightmap.update(&map_data, &entry.common, 0);
                displacement_lightmaps.insert(entry.face_index, lightmap);
            }
            GX_InvalidateTexAll();

            // Set up texture objects for the skybox (texture indices 0..5).
            let texture_data = map_data.texture_data();
            let skybox_texobjs: Vec<GXTexObj> = map_data.texture_table()[0..5]
                .iter()
                .map(|entry| {
                    let mut texobj = zeroed::<GXTexObj>();
                    GX_InitTexObj(
                        &mut texobj,
                        texture_data[entry.start_offset as usize..entry.end_offset as usize]
                            .as_ptr() as *mut c_void,
                        entry.width,
                        entry.height,
                        entry.format,
                        if (entry.flags & TextureTableEntry::FLAG_CLAMP_S) != 0 {
                            GX_CLAMP
                        } else {
                            GX_REPEAT
                        } as u8,
                        if (entry.flags & TextureTableEntry::FLAG_CLAMP_T) != 0 {
                            GX_CLAMP
                        } else {
                            GX_REPEAT
                        } as u8,
                        if entry.mip_count > 1 {
                            GX_TRUE
                        } else {
                            GX_FALSE
                        } as u8,
                    );
                    GX_InitTexObjLOD(
                        &mut texobj,
                        if entry.mip_count > 1 {
                            GX_LIN_MIP_LIN
                        } else {
                            GX_LINEAR
                        } as u8,
                        GX_LINEAR as u8,
                        0.0,
                        (entry.mip_count - 1) as f32,
                        0.0,
                        GX_ENABLE as u8,
                        GX_ENABLE as u8,
                        GX_ANISO_1 as u8,
                    );
                    texobj
                })
                .collect();

            let ui_font = {
                let mut ui_font = zeroed::<GXTexObj>();
                GX_InitTexObj(
                    &mut ui_font,
                    UI_FONT.as_ptr() as *mut c_void,
                    256,
                    256,
                    GX_TF_I8 as u8,
                    GX_CLAMP as u8,
                    GX_CLAMP as u8,
                    GX_FALSE as u8,
                );
                GX_InitTexObjLOD(
                    &mut ui_font,
                    GX_NEAR as u8,
                    GX_NEAR as u8,
                    0.0,
                    0.0,
                    0.0,
                    GX_FALSE as u8,
                    GX_FALSE as u8,
                    GX_ANISO_1 as u8,
                );
                ui_font
            };

            let visibility = Visibility::new(map_data.visibility().as_ptr());

            let mut game_state = GameState {
                // // d1_trainstation_01 classic view
                // pos: guVector {
                //     x: -4875.0,
                //     y: -1237.0,
                //     z: 140.0,
                // },
                // yaw: core::f32::consts::PI,
                // pitch: 0.0,

                // d1_trainstation_01 difficult case
                pos: guVector {
                    x: -4295.0,
                    y: -2543.0,
                    z: 140.0,
                },
                yaw: 3.6915,
                pitch: 0.0155,

                inverted_pitch_control: false,
                msaa: false,
                copy_filter: false,
                widescreen: get_widescreen_setting(),
                lightmap_style: 0,

                ui_item: 0,

                gp_perf_metric0: GpPerfMetric0::NONE,
                gp_perf_metric1: GpPerfMetric1::NONE,
            };

            let mut performance_metrics = PerformanceMetrics::default();
            let mut last_frame_timers = zeroed::<FrameTimers>();
            let mut last_frame_frames = 0;
            loop {
                match PENDING_GAME_STATE_CHANGE.load(Ordering::SeqCst) {
                    x if x == GameStateChange::Reset as u32 => {
                        libc::exit(0);
                    }
                    x if x == GameStateChange::Power as u32 => {
                        SYS_ResetSystem(SYS_POWEROFF as i32, 0, 0);
                    }
                    x if x == GameStateChange::MapSelect as u32 => {
                        break;
                    }
                    _ => (),
                }

                let game_logic_elapsed = Timer::time(|| {
                    do_game_logic(
                        &map_data,
                        &mut game_state,
                        &mut cluster_lightmaps,
                        &mut displacement_lightmaps,
                    );
                });
                let main_draw_elapsed = Timer::time(|| {
                    GX_ClearGPMetric();
                    GX_ClearVCacheMetric();

                    if game_state.msaa {
                        prepare_main_draw(width, height, &game_state, Some(false));
                        let view_cluster = do_main_draw(
                            &map_data,
                            &game_state,
                            visibility,
                            &skybox_texobjs,
                            &cluster_lightmaps,
                            &displacement_lightmaps,
                        );
                        do_debug_draw(
                            width,
                            height,
                            &game_state,
                            &last_frame_timers,
                            view_cluster,
                            &cluster_lightmaps,
                            &ui_font,
                            &performance_metrics,
                            last_frame_frames,
                        );
                        copy_disp(Some(false));

                        prepare_main_draw(width, height, &game_state, Some(true));
                        let view_cluster = do_main_draw(
                            &map_data,
                            &game_state,
                            visibility,
                            &skybox_texobjs,
                            &cluster_lightmaps,
                            &displacement_lightmaps,
                        );
                        do_debug_draw(
                            width,
                            height,
                            &game_state,
                            &last_frame_timers,
                            view_cluster,
                            &cluster_lightmaps,
                            &ui_font,
                            &performance_metrics,
                            last_frame_frames,
                        );
                        copy_disp(Some(true));
                    } else {
                        prepare_main_draw(width, height, &game_state, None);
                        let view_cluster = do_main_draw(
                            &map_data,
                            &game_state,
                            visibility,
                            &skybox_texobjs,
                            &cluster_lightmaps,
                            &displacement_lightmaps,
                        );
                        do_debug_draw(
                            width,
                            height,
                            &game_state,
                            &last_frame_timers,
                            view_cluster,
                            &cluster_lightmaps,
                            &ui_font,
                            &performance_metrics,
                            last_frame_frames,
                        );
                        copy_disp(None);
                    }
                });
                let copy_to_texture_elapsed = 0;
                let debug_draw_elapsed = 0;
                let draw_done_elapsed = Timer::time(|| {
                    GX_DrawDone();
                    performance_metrics = PerformanceMetrics::read();
                    DO_COPY.store(true, Ordering::Release);
                });
                let idle_elapsed = Timer::time(|| {
                    VIDEO_WaitVSync();
                });
                last_frame_frames = LAST_FRAME_FRAMES.load(Ordering::Acquire);

                last_frame_timers = FrameTimers {
                    game_logic: game_logic_elapsed,
                    main_draw: main_draw_elapsed,
                    copy_to_texture: copy_to_texture_elapsed,
                    debug_draw: debug_draw_elapsed,
                    draw_done: draw_done_elapsed,
                    idle: idle_elapsed,
                };
            }
        }
    }
}

struct GameState {
    pos: guVector,
    yaw: f32,
    pitch: f32,
    inverted_pitch_control: bool,
    msaa: bool,
    copy_filter: bool,
    widescreen: bool,
    lightmap_style: usize,

    ui_item: usize,

    gp_perf_metric0: GpPerfMetric0,
    gp_perf_metric1: GpPerfMetric1,
}

impl GameState {
    fn widescreen_factor(&self) -> f32 {
        if self.widescreen {
            4.0 / 3.0
        } else {
            1.0
        }
    }
}

pub struct Timer {
    start: u32,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: unsafe { gettick() },
        }
    }

    pub fn stop(self) -> u32 {
        unsafe { gettick() }.wrapping_sub(self.start)
    }

    pub fn time(f: impl FnOnce()) -> u32 {
        let timer = Self::start();
        f();
        timer.stop()
    }

    pub fn time_with_result<R>(f: impl FnOnce() -> R) -> (u32, R) {
        let timer = Self::start();
        let result = f();
        (timer.stop(), result)
    }
}

fn do_game_logic<Data: Deref<Target = [u8]>>(
    map_data: &MapData<Data>,
    game_state: &mut GameState,
    cluster_lightmaps: &mut [Lightmap],
    displacement_lightmaps: &mut BTreeMap<u16, Lightmap>,
) {
    unsafe {
        PAD_ScanPads();

        if (PAD_ButtonsDown(0) & PAD_BUTTON_START as u16) != 0 {
            PENDING_GAME_STATE_CHANGE.store(GameStateChange::MapSelect as u32, Ordering::SeqCst);
        }
        if (PAD_ButtonsDown(0) & PAD_TRIGGER_Z as u16) != 0 {
            game_state.inverted_pitch_control ^= true;
        }

        let right = [libm::sinf(game_state.yaw), -libm::cosf(game_state.yaw), 0.0];
        let forward = [libm::cosf(game_state.yaw), libm::sinf(game_state.yaw), 0.0];
        let speed = if PAD_TriggerR(0) >= 128 { 100.0 } else { 10.0 };
        let angspeed = 0.1;
        let (dx, dy) = get_processed_stick(0, false);
        let (cx, cy) = get_processed_stick(0, true);
        let cy = if game_state.inverted_pitch_control {
            -cy
        } else {
            cy
        };

        game_state.pos.x += speed * (right[0] * dx + forward[0] * dy);
        game_state.pos.y += speed * (right[1] * dx + forward[1] * dy);
        game_state.pos.z += speed * (right[2] * dx + forward[2] * dy);
        if (PAD_ButtonsHeld(0) & PAD_BUTTON_Y as u16) != 0 {
            game_state.pos.z += speed;
        }
        if (PAD_ButtonsHeld(0) & PAD_BUTTON_X as u16) != 0 {
            game_state.pos.z -= speed;
        }

        game_state.yaw -= angspeed * cx;
        game_state.pitch = (game_state.pitch + angspeed * cy).clamp(
            -89.0 / 180.0 * core::f32::consts::PI,
            89.0 / 180.0 * core::f32::consts::PI,
        );

        if (PAD_ButtonsDown(0) & PAD_BUTTON_UP as u16) != 0 {
            game_state.ui_item = game_state.ui_item.checked_sub(1).unwrap_or(4);
        }
        if (PAD_ButtonsDown(0) & PAD_BUTTON_DOWN as u16) != 0 {
            game_state.ui_item = (game_state.ui_item + 1) % 5;
        }

        let ui_increment: i32 = if (PAD_ButtonsDown(0) & PAD_BUTTON_LEFT as u16) != 0 {
            -1
        } else {
            0
        } + if (PAD_ButtonsDown(0) & PAD_BUTTON_RIGHT as u16) != 0 {
            1
        } else {
            0
        };

        match game_state.ui_item {
            0 => {
                game_state.msaa ^= ui_increment != 0;
            }

            1 => {
                game_state.copy_filter ^= ui_increment != 0;
            }

            2 => {
                // Change lightmap styles.
                let new_lightmap_style = game_state
                    .lightmap_style
                    .wrapping_add(ui_increment as usize)
                    % 4;

                if game_state.lightmap_style != new_lightmap_style {
                    game_state.lightmap_style = new_lightmap_style;
                    for (cluster_index, lightmap) in cluster_lightmaps.iter_mut().enumerate() {
                        let entry = &map_data.lightmap_cluster_table()[cluster_index];
                        lightmap.update(map_data, &entry.common, game_state.lightmap_style);
                    }
                    for entry in map_data.lightmap_displacement_table() {
                        displacement_lightmaps
                            .get_mut(&entry.face_index)
                            .unwrap()
                            .update(map_data, &entry.common, game_state.lightmap_style);
                    }
                }
            }

            3 => {
                // Change GP perf metric 0.
                match ui_increment {
                    -1 => game_state.gp_perf_metric0 = game_state.gp_perf_metric0.prev(),
                    1 => game_state.gp_perf_metric0 = game_state.gp_perf_metric0.next(),
                    _ => (),
                };
            }

            4 => {
                // Change GP perf metric 1.
                match ui_increment {
                    -1 => game_state.gp_perf_metric1 = game_state.gp_perf_metric1.prev(),
                    1 => game_state.gp_perf_metric1 = game_state.gp_perf_metric1.next(),
                    _ => (),
                };
            }

            _ => unreachable!(),
        }

        GX_SetGPMetric(
            game_state.gp_perf_metric0 as u32,
            game_state.gp_perf_metric1 as u32,
        );
        GX_SetVCacheMetric(GX_VC_ALL);
    }
}

fn prepare_main_draw(width: u16, height: u16, game_state: &GameState, half: Option<bool>) {
    unsafe {
        GX_SetPixelFmt(
            if game_state.msaa {
                GX_PF_RGB565_Z16
            } else {
                GX_PF_RGB8_Z24
            } as u8,
            GX_ZC_LINEAR as u8,
        );
        GX_SetCopyFilter(
            game_state.msaa as u8,
            TVNtsc480ProgAa.sample_pattern.as_ptr() as *mut [u8; 2],
            game_state.copy_filter as u8,
            TVNtsc480ProgSoft.vfilter.as_ptr() as *mut u8,
        );
        GX_SetDispCopyYScale(
            // Weird hack: Why does libogc set up a 242 line EFB in NTSC 480p AA mode?
            // TODO: Make this generally correct.
            1.0,
        );

        load_camera_proj_matrix(width, height, game_state, half);

        let mut eye_offset = zeroed::<Mtx>();
        c_guMtxTrans(
            eye_offset.as_mut_ptr(),
            -game_state.pos.x,
            -game_state.pos.y,
            -game_state.pos.z,
        );
        GX_LoadTexMtxImm(eye_offset.as_mut_ptr(), GX_TEXMTX0, GX_MTX3x4 as u8);

        let mut scale_and_bias = [[0.5, 0.0, 0.0, 0.5], [0.0, 0.5, 0.0, 0.5]];
        GX_LoadTexMtxImm(scale_and_bias.as_mut_ptr(), GX_TEXMTX1, GX_MTX2x4 as u8);

        let mut identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        GX_LoadNrmMtxImm3x3(identity.as_mut_ptr(), GX_PNMTX0);
    }
}

fn load_camera_proj_matrix(width: u16, height: u16, game_state: &GameState, half: Option<bool>) {
    unsafe {
        let mut proj = zeroed::<Mtx44>();
        guPerspective(
            proj.as_mut_ptr(),
            90.0,
            width as f32 / height as f32 * game_state.widescreen_factor(),
            1.0,
            16384.0,
        );
        GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_PERSPECTIVE as u8);

        GX_SetViewport(0.0, 0.0, 640.0, 480.0, 0.0, 1.0);
        match half {
            None => {
                GX_SetScissor(0, 0, 640, 480);
                GX_SetScissorBoxOffset(0, 0);
            }
            Some(false) => {
                GX_SetScissor(0, 0, 640, 240);
                GX_SetScissorBoxOffset(0, 0);
            }
            Some(true) => {
                GX_SetScissor(0, 240, 640, 240);
                GX_SetScissorBoxOffset(0, 240);
            }
        }
    }
}

fn load_camera_view_matrix(game_state: &GameState) {
    unsafe {
        let mut look_at = zeroed::<Mtx>();
        let mut yaw_rotation = zeroed::<Mtx>();
        let mut pitch_rotation = zeroed::<Mtx>();
        let mut tmp = zeroed::<Mtx>();
        guLookAt(
            look_at.as_mut_ptr(),
            &mut guVector {
                x: game_state.pos.x,
                y: game_state.pos.y,
                z: game_state.pos.z,
            },
            &mut guVector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            &mut guVector {
                x: game_state.pos.x + 1.0,
                y: game_state.pos.y,
                z: game_state.pos.z,
            },
        );
        c_guMtxRotRad(yaw_rotation.as_mut_ptr(), b'y', -game_state.yaw);
        c_guMtxRotRad(pitch_rotation.as_mut_ptr(), b'x', -game_state.pitch);
        c_guMtxConcat(
            yaw_rotation.as_mut_ptr(),
            look_at.as_mut_ptr(),
            tmp.as_mut_ptr(),
        );
        let mut view = zeroed::<Mtx>();
        c_guMtxConcat(
            pitch_rotation.as_mut_ptr(),
            tmp.as_mut_ptr(),
            view.as_mut_ptr(),
        );
        GX_LoadPosMtxImm(view.as_mut_ptr(), GX_PNMTX0);
    }
}

fn load_skybox_view_matrix(game_state: &GameState) {
    unsafe {
        let mut look_at = zeroed::<Mtx>();
        let mut yaw_rotation = zeroed::<Mtx>();
        let mut pitch_rotation = zeroed::<Mtx>();
        let mut tmp = zeroed::<Mtx>();
        guLookAt(
            look_at.as_mut_ptr(),
            &mut guVector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            &mut guVector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            &mut guVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        c_guMtxRotRad(yaw_rotation.as_mut_ptr(), b'y', -game_state.yaw);
        c_guMtxRotRad(pitch_rotation.as_mut_ptr(), b'x', -game_state.pitch);
        c_guMtxConcat(
            yaw_rotation.as_mut_ptr(),
            look_at.as_mut_ptr(),
            tmp.as_mut_ptr(),
        );
        let mut view = zeroed::<Mtx>();
        c_guMtxConcat(
            pitch_rotation.as_mut_ptr(),
            tmp.as_mut_ptr(),
            view.as_mut_ptr(),
        );
        GX_LoadPosMtxImm(view.as_mut_ptr(), GX_PNMTX0);
    }
}

fn do_main_draw<Data: Deref<Target = [u8]>>(
    map_data: &MapData<Data>,
    game_state: &GameState,
    visibility: Visibility,
    skybox_texobjs: &[GXTexObj],
    cluster_lightmaps: &[Lightmap],
    displacement_lightmaps: &BTreeMap<u16, Lightmap>,
) -> i16 {
    draw_skybox(game_state, skybox_texobjs);
    draw_displacements(map_data, game_state, displacement_lightmaps);
    let view_cluster = draw_visible_clusters(map_data, game_state, cluster_lightmaps, visibility);
    view_cluster
}

fn draw_visible_clusters<Data: Deref<Target = [u8]>>(
    map_data: &MapData<Data>,
    game_state: &GameState,
    cluster_lightmaps: &[Lightmap],
    visibility: Visibility,
) -> i16 {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_NRM as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_DIRECT as u8);
        GX_SetVtxDesc(GX_VA_TEX1 as u8, GX_INDEX16 as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_NRM, GX_NRM_XYZ, GX_S8, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_U16, 15);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX1, GX_TEX_ST, GX_U16, 8);
        GX_SetArray(GX_VA_POS, map_data.position_data().as_ptr() as *mut _, 12);
        GX_SetArray(GX_VA_NRM, map_data.normal_data().as_ptr() as *mut _, 3);
        GX_SetArray(
            GX_VA_TEX1,
            map_data.texture_coord_data().as_ptr() as *mut _,
            4,
        );
        GX_InvVtxCache();

        GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);

        let view_leaf =
            map_data.traverse_bsp(&[game_state.pos.x, game_state.pos.y, game_state.pos.z]);
        let view_cluster = view_leaf.cluster;

        // Memoize some map data sections.
        let cluster_geometry_table = map_data.cluster_geometry_table();
        let cluster_geometry_byte_code = map_data.cluster_geometry_byte_code();
        let cluster_geometry_display_lists = map_data.cluster_geometry_display_lists();

        let draw_cluster = move |cluster: u16, pass: usize| {
            let cluster_geometry = cluster_geometry_table[cluster as usize];
            // Bind the lightmap, but only if there's rendering to be done.
            if cluster_geometry.byte_code_index_ranges[pass][0]
                != cluster_geometry.byte_code_index_ranges[pass][1]
            {
                match cluster_lightmaps.get(cluster as usize) {
                    Some(lightmap) => {
                        GX_LoadTexObj(lightmap.texobj(), GX_TEXMAP0 as u8);
                    }
                    None => return,
                }
            }
            for entry in cluster_geometry.iter_display_lists(cluster_geometry_byte_code, pass) {
                match entry {
                    BytecodeOp::Draw {
                        display_list_offset,
                        display_list_size,
                    } => {
                        GX_CallDispList(
                            (cluster_geometry_display_lists.as_ptr() as *mut c_void)
                                .offset(display_list_offset as isize),
                            display_list_size,
                        );
                    }
                    BytecodeOp::SetVertexDesc { attr_list_offset } => {
                        panic!();
                        GX_ClearVtxDesc();
                        GX_SetVtxDescv(null_mut());
                    }
                    BytecodeOp::SetAlphaCompare {
                        z_comp_before_tex,
                        compare_type,
                        reference,
                    } => {
                        GX_SetZCompLoc(z_comp_before_tex);
                        GX_SetAlphaCompare(
                            compare_type,
                            reference,
                            GX_AOP_AND as u8,
                            GX_ALWAYS as u8,
                            0,
                        );
                    }
                    BytecodeOp::SetFaceIndex { .. } => unreachable!(),
                }
            }
        };

        for pass in 0..6 {
            if pass < 4 {
                match pass & 0x1 {
                    0 => LIGHTMAPPED_SHADER.apply(),
                    1 => LIGHTMAPPED_BAAA_SHADER.apply(),
                    _ => unreachable!(),
                }
            } else if pass == 4 {
                UNLIT_GENERIC_SHADER.apply();
            } else if pass == 5 {
                SELF_ILLUM_SHADER.apply();
            }

            let blend = pass < 4 && (pass & 2) == 2;
            if blend {
                // Alpha blending.
                GX_SetBlendMode(
                    GX_BM_BLEND as u8,
                    GX_BL_SRCALPHA as u8,
                    GX_BL_INVSRCALPHA as u8,
                    0,
                );
                GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_FALSE as u8);
            } else {
                // Blending off.
                GX_SetBlendMode(GX_BM_NONE as u8, 0, 0, 0);
                GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);
            }

            if view_cluster != -1 {
                for cluster in visibility
                    .get_cluster(ClusterIndex(view_cluster as usize))
                    .iter_visible_clusters()
                    .map(|cluster| cluster.0 as u16)
                {
                    draw_cluster(cluster, pass);
                }
            } else {
                for cluster in 0..visibility.num_clusters() as u16 {
                    draw_cluster(cluster, pass);
                }
            }
        }

        GX_SetBlendMode(GX_BM_NONE as u8, 0, 0, 0);
        GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);
        GX_SetZCompLoc(GX_TRUE as u8);
        GX_SetAlphaCompare(GX_ALWAYS as u8, 0, GX_AOP_OR as u8, GX_ALWAYS as u8, 0);

        view_cluster
    }
}

fn draw_skybox(game_state: &GameState, skybox_texobjs: &[GXTexObj]) {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_DIRECT as u8);
        GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_DIRECT as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XYZ, GX_S8, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_U8, 0);
        GX_InvVtxCache();

        load_skybox_view_matrix(game_state);

        GX_SetZMode(GX_FALSE as u8, GX_ALWAYS as u8, GX_FALSE as u8);
        GX_SetColorUpdate(GX_TRUE as u8);

        FLAT_TEXTURED_SHADER.apply();

        // +X face.
        GX_LoadTexObj(
            &skybox_texobjs[0] as *const GXTexObj as *mut GXTexObj,
            GX_TEXMAP0 as u8,
        );
        GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);
        {
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 1;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 1;
        }

        // -X face.
        GX_LoadTexObj(
            &skybox_texobjs[1] as *const GXTexObj as *mut GXTexObj,
            GX_TEXMAP0 as u8,
        );
        GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);
        {
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 1;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 1;
        }

        // +Y face.
        GX_LoadTexObj(
            &skybox_texobjs[2] as *const GXTexObj as *mut GXTexObj,
            GX_TEXMAP0 as u8,
        );
        GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);
        {
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 1;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 1;
        }

        // -Y face.
        GX_LoadTexObj(
            &skybox_texobjs[3] as *const GXTexObj as *mut GXTexObj,
            GX_TEXMAP0 as u8,
        );
        GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);
        {
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 1;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 1;
        }

        // +Z face.
        GX_LoadTexObj(
            &skybox_texobjs[4] as *const GXTexObj as *mut GXTexObj,
            GX_TEXMAP0 as u8,
        );
        GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);
        {
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 0;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = -10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 1;

            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).S8 = 10;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 1;
        }
    }
}

fn draw_displacements<Data: Deref<Target = [u8]>>(
    map_data: &MapData<Data>,
    game_state: &GameState,
    displacement_lightmaps: &BTreeMap<u16, Lightmap>,
) {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_CLR0 as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_DIRECT as u8);
        GX_SetVtxDesc(GX_VA_TEX1 as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX2 as u8, GX_INDEX16 as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_CLR0, GX_CLR_RGB, GX_RGB8, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_U16, 15);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX1, GX_TEX_ST, GX_U16, 8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX2, GX_TEX_ST, GX_U16, 8);
        GX_SetArray(
            GX_VA_POS,
            map_data.displacement_position_data().as_ptr() as *mut _,
            12,
        );
        GX_SetArray(
            GX_VA_CLR0,
            map_data.displacement_vertex_color_data().as_ptr() as *mut _,
            3,
        );
        GX_SetArray(
            GX_VA_TEX1,
            map_data.displacement_texture_coordinate_data().as_ptr() as *mut _,
            4,
        );
        GX_SetArray(
            GX_VA_TEX2,
            map_data.displacement_texture_coordinate_data().as_ptr() as *mut _,
            4,
        );
        GX_InvVtxCache();

        load_camera_view_matrix(game_state);

        GX_SetBlendMode(GX_BM_NONE as u8, 0, 0, 0);
        GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);

        let displacement_byte_code = map_data.displacement_byte_code();
        let displacement_display_lists = map_data.displacement_display_lists();

        let mut prev_mode = None;
        for (mode, entry) in map_data.displacement_table().iter().enumerate() {
            if prev_mode != Some(mode) {
                prev_mode = Some(mode);
                match mode {
                    0 => LIGHTMAPPED_SHADER.apply(),
                    1 => WORLD_VERTEX_TRANSITION_SHADER.apply(),
                    _ => unreachable!(),
                }
            }

            for op in BytecodeReader::new(
                &displacement_byte_code
                    [entry.byte_code_start_index as usize..entry.byte_code_end_index as usize],
            ) {
                match op {
                    BytecodeOp::Draw {
                        display_list_offset,
                        display_list_size,
                    } => {
                        GX_CallDispList(
                            (displacement_display_lists.as_ptr() as *mut c_void)
                                .offset(display_list_offset as isize),
                            display_list_size,
                        );
                    }
                    BytecodeOp::SetFaceIndex { face_index } => {
                        GX_LoadTexObj(
                            displacement_lightmaps[&face_index].texobj(),
                            GX_TEXMAP0 as u8,
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

fn _do_copy_to_texture(screen_texture_color_data: &Vec<u8, GlobalAlign32>) {
    unsafe {
        // Copy the color buffer to a texture in main memory.
        GX_SetTexCopySrc(0, 0, 640, 480); // TODO: Use the current mode.

        DCInvalidateRange(
            screen_texture_color_data.as_ptr() as _,
            screen_texture_color_data.len() as u32,
        );
        GX_SetTexCopyDst(640, 480, GX_TF_RGBA8, GX_FALSE as u8);
        GX_CopyTex(screen_texture_color_data.as_ptr() as _, GX_FALSE as u8);

        GX_PixModeSync();
        GX_Flush();
        GX_DrawDone();

        // Draw with the color buffer texture.

        // LIGHTMAPPED_REFLECTIVE_SHADER.apply();

        GX_InvalidateTexAll();

        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_DIRECT as u8);
        GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_DIRECT as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XY, GX_U16, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_U8, 0);

        GX_SetZMode(GX_FALSE as u8, GX_ALWAYS as u8, GX_FALSE as u8);
        GX_SetColorUpdate(GX_TRUE as u8);

        let mut proj = zeroed::<Mtx44>();
        guOrtho(proj.as_mut_ptr(), 0.0, 1.0, 0.0, 1.0, -1.0, 1.0);
        GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_ORTHOGRAPHIC as u8);

        let mut view = zeroed::<Mtx>();
        c_guMtxIdentity(view.as_mut_ptr());
        GX_LoadPosMtxImm(view.as_mut_ptr(), GX_PNMTX0);

        GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);

        (*wgPipe).U16 = 0;
        (*wgPipe).U16 = 0;
        (*wgPipe).U8 = 0;
        (*wgPipe).U8 = 0;

        (*wgPipe).U16 = 1;
        (*wgPipe).U16 = 0;
        (*wgPipe).U8 = 1;
        (*wgPipe).U8 = 0;

        (*wgPipe).U16 = 1;
        (*wgPipe).U16 = 1;
        (*wgPipe).U8 = 1;
        (*wgPipe).U8 = 1;

        (*wgPipe).U16 = 0;
        (*wgPipe).U16 = 1;
        (*wgPipe).U8 = 0;
        (*wgPipe).U8 = 1;
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(u32)]
enum GpPerfMetric0 {
    VERTICES = 0,
    CLIP_VTX = 1,
    CLIP_CLKS = 2,
    XF_WAIT_IN = 3,
    XF_WAIT_OUT = 4,
    XF_XFRM_CLKS = 5,
    XF_LIT_CLKS = 6,
    XF_BOT_CLKS = 7,
    XF_REGLD_CLKS = 8,
    XF_REGRD_CLKS = 9,
    CLIP_RATIO = 10,
    TRIANGLES = 11,
    TRIANGLES_CULLED = 12,
    TRIANGLES_PASSED = 13,
    TRIANGLES_SCISSORED = 14,
    TRIANGLES_0TEX = 15,
    TRIANGLES_1TEX = 16,
    TRIANGLES_2TEX = 17,
    TRIANGLES_3TEX = 18,
    TRIANGLES_4TEX = 19,
    TRIANGLES_5TEX = 20,
    TRIANGLES_6TEX = 21,
    TRIANGLES_7TEX = 22,
    TRIANGLES_8TEX = 23,
    TRIANGLES_0CLR = 24,
    TRIANGLES_1CLR = 25,
    TRIANGLES_2CLR = 26,
    QUAD_0CVG = 27,
    QUAD_NON0CVG = 28,
    QUAD_1CVG = 29,
    QUAD_2CVG = 30,
    QUAD_3CVG = 31,
    QUAD_4CVG = 32,
    AVG_QUAD_CNT = 33,
    CLOCKS = 34,
    NONE = 35,
}

impl GpPerfMetric0 {
    fn prev(self) -> Self {
        if let Ok(result) = Self::try_from(self as u32 - 1) {
            result
        } else {
            Self::NONE
        }
    }

    fn next(self) -> Self {
        if let Ok(result) = Self::try_from(self as u32 + 1) {
            result
        } else {
            Self::VERTICES
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(u32)]
enum GpPerfMetric1 {
    TEXELS = 0,
    TX_IDLE = 1,
    TX_REGS = 2,
    TX_MEMSTALL = 3,
    TC_CHECK1_2 = 4,
    TC_CHECK3_4 = 5,
    TC_CHECK5_6 = 6,
    TC_CHECK7_8 = 7,
    TC_MISS = 8,
    VC_ELEMQ_FULL = 9,
    VC_MISSQ_FULL = 10,
    VC_MEMREQ_FULL = 11,
    VC_STATUS7 = 12,
    VC_MISSREP_FULL = 13,
    VC_STREAMBUF_LOW = 14,
    VC_ALL_STALLS = 15,
    VERTICES = 16,
    FIFO_REQ = 17,
    CALL_REQ = 18,
    VC_MISS_REQ = 19,
    CP_ALL_REQ = 20,
    CLOCKS = 21,
    NONE = 22,
}

impl GpPerfMetric1 {
    fn prev(self) -> Self {
        if let Ok(result) = Self::try_from(self as u32 - 1) {
            result
        } else {
            Self::NONE
        }
    }

    fn next(self) -> Self {
        if let Ok(result) = Self::try_from(self as u32 + 1) {
            result
        } else {
            Self::TEXELS
        }
    }
}

#[derive(Default)]
struct PerformanceMetrics {
    gp_a: u32,
    gp_b: u32,
    gp_c: u32,
    gp_d: u32,
    vcache_metric_check: u32,
    vcache_metric_miss: u32,
    vcache_metric_stall: u32,
}

impl PerformanceMetrics {
    fn read() -> Self {
        unsafe {
            let cp = gamecube_peripheral_access::CP::PTR;
            let gp_a = ((*cp).xf_rasbusy_h.read().bits() as u32) << 16
                | (*cp).xf_rasbusy_l.read().bits() as u32;
            let gp_b =
                ((*cp).xf_clks_h.read().bits() as u32) << 16 | (*cp).xf_clks_l.read().bits() as u32;
            let gp_c = ((*cp).xf_wait_in_h.read().bits() as u32) << 16
                | (*cp).xf_wait_in_l.read().bits() as u32;
            let gp_d = ((*cp).xf_wait_out_h.read().bits() as u32) << 16
                | (*cp).xf_wait_out_l.read().bits() as u32;
            let vcache_metric_check = ((*cp).vcache_metric_check_h.read().bits() as u32) << 16
                | (*cp).vcache_metric_check_l.read().bits() as u32;
            let vcache_metric_miss = ((*cp).vcache_metric_miss_h.read().bits() as u32) << 16
                | (*cp).vcache_metric_miss_l.read().bits() as u32;
            let vcache_metric_stall = ((*cp).vcache_metric_stall_h.read().bits() as u32) << 16
                | (*cp).vcache_metric_stall_l.read().bits() as u32;
            // let clks_per_vtx_in = ((*cp).clks_per_vtx_in_h.read().bits() as u32) << 16
            //     | (*cp).clks_per_vtx_in_l.read().bits() as u32;
            // let clks_per_vtx_out = (*cp).clks_per_vtx_out.read().bits() as u32;
            Self {
                gp_a,
                gp_b,
                gp_c,
                gp_d,
                vcache_metric_check,
                vcache_metric_miss,
                vcache_metric_stall,
                // clks_per_vtx_in: 0,
                // clks_per_vtx_out: 0,
            }
        }
    }
}

fn do_debug_draw(
    width: u16,
    height: u16,
    game_state: &GameState,
    last_frame_timers: &FrameTimers,
    view_cluster: i16,
    cluster_lightmaps: &[Lightmap],
    ui_font: &GXTexObj,
    performance_metrics: &PerformanceMetrics,
    last_frame_frames: usize,
) {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_DIRECT as u8);
        GX_SetVtxDesc(GX_VA_CLR0 as u8, GX_DIRECT as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XY, GX_U16, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_CLR0, GX_CLR_RGB, GX_RGB8, 0);
        GX_InvVtxCache();

        FLAT_VERTEX_COLOR_SHADER.apply();

        GX_SetZMode(GX_FALSE as u8, GX_ALWAYS as u8, GX_FALSE as u8);
        GX_SetColorUpdate(GX_TRUE as u8);

        let mut proj = zeroed::<Mtx44>();
        guOrtho(
            proj.as_mut_ptr(),
            0.0,
            height as f32,
            0.0,
            width as f32,
            -1.0,
            1.0,
        );
        GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_ORTHOGRAPHIC as u8);

        let mut view = zeroed::<Mtx>();
        c_guMtxIdentity(view.as_mut_ptr());
        GX_LoadPosMtxImm(view.as_mut_ptr(), GX_PNMTX0);

        let to_y = height - 16;
        let from_y = to_y - 16;
        let emit_debug_quad = |from_x, to_x, max_x, r, g, b| {
            let from_x = (from_x as f32 * width as f32 / max_x as f32) as u16;
            let to_x = (to_x as f32 * width as f32 / max_x as f32) as u16;

            GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);

            (*wgPipe).U16 = from_x;
            (*wgPipe).U16 = from_y;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;

            (*wgPipe).U16 = to_x;
            (*wgPipe).U16 = from_y;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;

            (*wgPipe).U16 = to_x;
            (*wgPipe).U16 = to_y;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;

            (*wgPipe).U16 = from_x;
            (*wgPipe).U16 = to_y;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;
        };

        let x0 = 0;
        let x1 = x0 + last_frame_timers.game_logic;
        let x2 = x1 + last_frame_timers.main_draw;
        let x3 = x2 + last_frame_timers.copy_to_texture;
        let x4 = x3 + last_frame_timers.debug_draw;
        let x5 = x4 + last_frame_timers.draw_done;
        let x6 = x5 + last_frame_timers.idle;

        emit_debug_quad(x0, x1, x6, 255, 0, 0);
        emit_debug_quad(x1, x2, x6, 255, 128, 0);
        emit_debug_quad(x2, x3, x6, 255, 255, 0);
        emit_debug_quad(x3, x4, x6, 0, 255, 0);
        emit_debug_quad(x4, x5, x6, 0, 0, 255);
        emit_debug_quad(x5, x6, x6, 255, 0, 255);

        let draw_bit = |x0, y0, bit| {
            let x1 = x0 + 16;
            let y1 = y0 + 16;
            let (r, g, b) = if bit { (0, 255, 0) } else { (0, 64, 0) };

            GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);

            (*wgPipe).U16 = x0;
            (*wgPipe).U16 = y0;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;

            (*wgPipe).U16 = x1;
            (*wgPipe).U16 = y0;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;

            (*wgPipe).U16 = x1;
            (*wgPipe).U16 = y1;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;

            (*wgPipe).U16 = x0;
            (*wgPipe).U16 = y1;
            (*wgPipe).U8 = r;
            (*wgPipe).U8 = g;
            (*wgPipe).U8 = b;
        };
        draw_bit(16, 16, view_cluster != -1);
        draw_bit(16, 0, (game_state.lightmap_style & 2) != 0);
        draw_bit(32, 0, (game_state.lightmap_style & 1) != 0);

        // Draw the lightmap for the current cluster.
        if let Some(lightmap) = cluster_lightmaps.get(view_cluster as usize) {
            let w = GX_GetTexObjWidth(lightmap.texobj());
            let h = GX_GetTexObjHeight(lightmap.texobj());

            GX_ClearVtxDesc();
            GX_SetVtxDesc(GX_VA_POS as u8, GX_DIRECT as u8);
            GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_DIRECT as u8);
            GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XY, GX_U16, 0);
            GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_U8, 0);
            GX_InvVtxCache();

            FLAT_TEXTURED_SHADER.apply();

            {
                let data = GX_GetTexObjData(lightmap.texobj());
                let format = GX_GetTexObjFmt(lightmap.texobj()) as u8;
                let width = GX_GetTexObjWidth(lightmap.texobj());
                let height = GX_GetTexObjHeight(lightmap.texobj());
                let mut dst = zeroed::<GXTexObj>();
                GX_InitTexObj(
                    &mut dst,
                    data,
                    width,
                    height,
                    format,
                    GX_CLAMP as u8,
                    GX_CLAMP as u8,
                    GX_FALSE as u8,
                );
                GX_InitTexObjFilterMode(&mut dst, GX_NEAR as u8, GX_NEAR as u8);
                GX_LoadTexObj(&mut dst, GX_TEXMAP0 as u8);
            }

            GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);

            (*wgPipe).U16 = 16;
            (*wgPipe).U16 = 16;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 0;

            (*wgPipe).U16 = 16 + w;
            (*wgPipe).U16 = 16;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 0;

            (*wgPipe).U16 = 16 + w;
            (*wgPipe).U16 = 16 + h;
            (*wgPipe).U8 = 1;
            (*wgPipe).U8 = 1;

            (*wgPipe).U16 = 16;
            (*wgPipe).U16 = 16 + h;
            (*wgPipe).U8 = 0;
            (*wgPipe).U8 = 1;
        }

        // Draw some  T E X T
        TextRenderer::prepare(ui_font);
        let mut r = TextRenderer {
            x: 16,
            y: 480 - 15 * 16,
            left_margin: 16,
        };
        let buf = format!(
            "At ({}, {}, {}) yaw={} pitch={}\n\
             {} MSAA: {}\n\
             {} Copy filter: {}\n\
             {} Lightmap style: {}\n\
             {} GP perf metric 0: {:?}\n\
             {} GP perf metric 1: {:?}\n\
             gp_a: {}\n\
             gp_b: {}\n\
             gp_c: {}\n\
             gp_d: {}\n\
             vcache_metric_check: {}\n\
             vcache_metric_miss: {}\n\
             vcache_metric_stall: {}\n",
            game_state.pos.x.round(),
            game_state.pos.y.round(),
            game_state.pos.z.round(),
            game_state.yaw,
            game_state.pitch,
            if game_state.ui_item == 0 { "->" } else { "  " },
            game_state.msaa,
            if game_state.ui_item == 1 { "->" } else { "  " },
            game_state.copy_filter,
            if game_state.ui_item == 2 { "->" } else { "  " },
            game_state.lightmap_style,
            if game_state.ui_item == 3 { "->" } else { "  " },
            game_state.gp_perf_metric0,
            if game_state.ui_item == 4 { "->" } else { "  " },
            game_state.gp_perf_metric1,
            performance_metrics.gp_a,
            performance_metrics.gp_b,
            performance_metrics.gp_c,
            performance_metrics.gp_d,
            performance_metrics.vcache_metric_check,
            performance_metrics.vcache_metric_miss,
            performance_metrics.vcache_metric_stall,
        );
        r.draw_str(buf.as_bytes());
        r.x = 640 - 24;
        r.y = 480 - 28;
        let buf = format!("{}", last_frame_frames);
        r.draw_str(buf.as_bytes());
    }
}

fn init_for_console() -> (*mut GXRModeObj, u16, u16) {
    unsafe {
        VIDEO_Init();
        PAD_Init();

        // Configure the preferred video mode.
        // let rmode = VIDEO_GetPreferredMode(null_mut());
        let rmode = &TVNtsc480ProgAa as *const GXRModeObj as _;
        VIDEO_Configure(rmode);

        // Allocate an external frame buffer, set up a vblank callback to swap buffers, and wait two
        // frames (for hardware to warm up?).
        let mut xfb_front = XFB_FRONT.load(Ordering::Acquire);
        if xfb_front.is_null() {
            xfb_front = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
            XFB_FRONT.store(xfb_front, Ordering::Release);
            let xfb_back = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
            XFB_BACK.store(xfb_back, Ordering::Release);
        }
        VIDEO_ClearFrameBuffer(rmode, xfb_front, 0x80808080);
        VIDEO_SetNextFramebuffer(xfb_front);
        VIDEO_SetBlack(false);
        VIDEO_Flush();
        VIDEO_WaitVSync();
        if ((*rmode).viTVMode & VI_NON_INTERLACE) != 0 {
            VIDEO_WaitVSync();
        }

        CON_InitEx(
            rmode,
            16,
            16,
            (*rmode).fbWidth as i32 - 32,
            (*rmode).xfbHeight as i32 - 32,
        );

        libc::printf(b"Inception\n\n\0".as_ptr());

        SYS_SetResetCallback(Some(on_reset_pressed));
        #[cfg(feature = "wii")]
        {
            SYS_SetPowerCallback(Some(on_power_pressed));
        }

        (rmode, (*rmode).fbWidth, (*rmode).efbHeight)
    }
}

/// Assumes init_for_console() was called previously.
fn init_for_3d(rmode: &GXRModeObj) {
    unsafe {
        drop(VIDEO_SetPreRetraceCallback(Some(pre_retrace_callback)));
        drop(VIDEO_SetPostRetraceCallback(None));

        // Allocate a FIFO for sending commands to the GPU.
        let gp_fifo = GP_FIFO.load(Ordering::Acquire);
        if gp_fifo.is_null() {
            const FIFO_SIZE: usize = 512 * 1024;
            let gp_fifo = MEM_K0_TO_K1(libc::memalign(32, FIFO_SIZE));
            GP_FIFO.store(gp_fifo, Ordering::Release);
            libc::memset(gp_fifo, 0, FIFO_SIZE);
            GX_Init(gp_fifo, FIFO_SIZE as u32);
        }

        GX_SetCopyClear(
            GXColor {
                r: 0x80,
                g: 0x80,
                b: 0x80,
                a: 0xff,
            },
            0x00ffffff,
        );
        GX_SetFieldMode(
            rmode.field_rendering,
            if rmode.viHeight == 2 * rmode.xfbHeight {
                GX_ENABLE
            } else {
                GX_DISABLE
            } as u8,
        );

        GX_SetCullMode(GX_CULL_BACK as u8);
        GX_SetDispCopyGamma(GX_GM_1_0 as u8);

        // Custom texture cache configuration: statically reserve 1/4 of each bank for textures 0-3.
        GX_InitTexCacheRegion(
            &mut TEX_REGIONS[0],
            GX_FALSE as u8,
            0,
            GX_TEXCACHE_128K as u8,
            512 * 1024,
            GX_TEXCACHE_128K as u8,
        );
        GX_InitTexCacheRegion(
            &mut TEX_REGIONS[1],
            GX_FALSE as u8,
            128 * 1024,
            GX_TEXCACHE_128K as u8,
            640 * 1024,
            GX_TEXCACHE_128K as u8,
        );
        GX_InitTexCacheRegion(
            &mut TEX_REGIONS[2],
            GX_FALSE as u8,
            256 * 1024,
            GX_TEXCACHE_128K as u8,
            768 * 1024,
            GX_TEXCACHE_128K as u8,
        );
        GX_InitTexCacheRegion(
            &mut TEX_REGIONS[3],
            GX_FALSE as u8,
            384 * 1024,
            GX_TEXCACHE_128K as u8,
            896 * 1024,
            GX_TEXCACHE_128K as u8,
        );
        GX_SetTexRegionCallback(Some(tex_region_callback));
    }
}

static mut TEX_REGIONS: [GXTexRegion; 4] = [GXTexRegion { val: [0; 4] }; 4];

unsafe extern "C" fn tex_region_callback(_obj: *mut GXTexObj, map_id: u8) -> *mut GXTexRegion {
    unsafe {
        assert!(map_id < 4);
        &mut TEX_REGIONS[map_id as usize]
    }
}

#[repr(u32)]
enum GameStateChange {
    None,
    Reset,
    Power,
    MapSelect,
}

static PENDING_GAME_STATE_CHANGE: AtomicU32 = AtomicU32::new(GameStateChange::None as u32);

unsafe extern "C" fn on_reset_pressed(_irq: u32, _ctx: *mut c_void) {
    PENDING_GAME_STATE_CHANGE.store(GameStateChange::Reset as u32, Ordering::SeqCst);
}

#[cfg(feature = "wii")]
unsafe extern "C" fn on_power_pressed() {
    PENDING_GAME_STATE_CHANGE.store(GameStateChange::Power as u32, Ordering::SeqCst);
}

struct FrameTimers {
    game_logic: u32,
    main_draw: u32,
    copy_to_texture: u32,
    debug_draw: u32,
    draw_done: u32,
    idle: u32,
}

extern "C" fn pre_retrace_callback(_count: u32) {
    FRAMES.fetch_add(1, Ordering::AcqRel);

    if DO_COPY
        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        // Swap buffers.
        let next_xfb_back = XFB_FRONT.load(Ordering::Acquire);
        let next_xfb_front = XFB_BACK.load(Ordering::Acquire);
        XFB_BACK.store(next_xfb_back, Ordering::Release);
        XFB_FRONT.store(next_xfb_front, Ordering::Release);
        unsafe {
            VIDEO_SetNextFramebuffer(next_xfb_front);
            VIDEO_Flush();
        }

        LAST_FRAME_FRAMES.store(FRAMES.swap(0, Ordering::AcqRel), Ordering::Release);
    }
}

fn copy_disp(half: Option<bool>) {
    unsafe {
        GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);
        GX_SetColorUpdate(GX_TRUE as u8);
        match half {
            None => {
                GX_SetDispCopySrc(0, 0, 640, 480);
                GX_SetDispCopyDst(640, 480);
                GX_CopyDisp(XFB_BACK.load(Ordering::Acquire), GX_TRUE as u8);
            }
            Some(false) => {
                GX_SetDispCopySrc(0, 0, 640, 240);
                GX_SetDispCopyDst(640, 240);
                GX_CopyDisp(XFB_BACK.load(Ordering::Acquire), GX_TRUE as u8);
            }
            Some(true) => {
                GX_SetDispCopySrc(0, 0, 640, 240);
                GX_SetDispCopyDst(640, 240);
                GX_CopyDisp(
                    (XFB_BACK.load(Ordering::Acquire) as usize + 2 * 640 * 240) as _,
                    GX_TRUE as u8,
                );
            }
        }
        GX_Flush();
    }
}

fn get_processed_stick(pad: i32, c: bool) -> (f32, f32) {
    let dx = if c {
        unsafe { PAD_SubStickX(pad) }
    } else {
        unsafe { PAD_StickX(0) }
    } as f32
        / 127.0;
    let dy = if c {
        unsafe { PAD_SubStickY(pad) }
    } else {
        unsafe { PAD_StickY(0) }
    } as f32
        / 127.0;
    let d = libm::sqrtf(dx * dx + dy * dy);
    if d < 0.2 {
        (0.0, 0.0)
    } else if d < 0.9 {
        let goal = (d - 0.1) * (1.0 / 0.8);
        let scale = goal / d;
        (dx * scale, dy * scale)
    } else {
        let scale = 1.0 / d;
        (dx * scale, dy * scale)
    }
}
