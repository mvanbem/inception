#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(allocator_api)]
#![feature(const_fn_trait_bound)]
#![feature(core_intrinsics)]
#![feature(default_alloc_error_handler)]
#![feature(start)]
#![feature(vec_spare_capacity)]

extern crate alloc;
#[cfg(test)]
extern crate std;

use core::ffi::c_void;
use core::mem::zeroed;
use core::ops::Deref;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inception_render_common::bytecode::{BytecodeOp, BytecodeReader};
use inception_render_common::map_data::{MapData, TextureTableEntry};
use ogc_sys::*;

use crate::lightmap::Lightmap;
use crate::loader::Loader;
use crate::memalign::Memalign;
use crate::shaders::flat_textured::FLAT_TEXTURED_SHADER;
use crate::shaders::flat_vertex_color::FLAT_VERTEX_COLOR_SHADER;
use crate::shaders::lightmapped::LIGHTMAPPED_SHADER;
use crate::shaders::lightmapped_baaa::LIGHTMAPPED_BAAA_SHADER;
use crate::shaders::lightmapped_baaa_env::LIGHTMAPPED_BAAA_ENV_SHADER;
use crate::shaders::lightmapped_baaa_env_emai::LIGHTMAPPED_BAAA_ENV_EMAI_SHADER;
use crate::shaders::lightmapped_env::LIGHTMAPPED_ENV_SHADER;
use crate::shaders::lightmapped_env_emai::LIGHTMAPPED_ENV_EMAI_SHADER;
use crate::shaders::unlit_generic::UNLIT_GENERIC_SHADER;
use crate::shaders::world_vertex_transition::WORLD_VERTEX_TRANSITION_SHADER;
use crate::visibility::{ClusterIndex, Visibility};

mod gx;
#[macro_use]
mod include_bytes_align;
mod iso9660;
mod lightmap;
mod loader;
mod memalign;
mod net;
mod shader;
mod shaders;
mod visibility;

static XFB: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static GP_FIFO: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static DO_COPY: AtomicBool = AtomicBool::new(false);

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
        return crate::loader::dvd_gcm_loader::DvdGcmLoader::new(());
    }

    #[cfg(feature = "ftp_loader")]
    {
        return crate::loader::ftp_loader::FtpLoader::new(crate::net::SocketAddr::new(
            [10, 0, 1, 105],
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

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        init_for_console();

        loop {
            libc::printf(b"Resetting the disc drive...\n\0".as_ptr());
            gamecube_dvd_driver::reset();
            match gamecube_dvd_driver::read_disc_id() {
                Ok(disc_id) => {
                    if &disc_id[..8] == b"GGMEMV\x00\x00" {
                        break;
                    }
                }
                Err(_) => (),
            }

            libc::printf(b"Unrecognized disc. Open the disc cover.\n\0".as_ptr());
            gamecube_dvd_driver::wait_for_cover(true);

            libc::printf(b"Insert the Inception disc and close the cover.\n\0".as_ptr());
            gamecube_dvd_driver::wait_for_cover(false);
        }

        let mut loader = configure_loader();

        loop {
            PENDING_GAME_STATE_CHANGE.store(GameStateChange::None as u32, Ordering::SeqCst);

            let (rmode, width, height) = init_for_console();

            let map = select_map(&mut loader);
            libc::printf(b"Loading map...\n\0".as_ptr());
            let map_data = loader.load_map(&map);

            init_for_3d(&*rmode);

            // // Set up full screen render to texture.
            // let screen_texture_color_data = Memalign::new(
            //     32,
            //     GX_GetTexBufferSize(640, 480, GX_TF_RGBA8, GX_FALSE as u8, 0) as usize,
            // );
            // {
            //     let mut screen_texture_color_texobj = zeroed::<GXTexObj>();
            //     GX_InitTexObj(
            //         &mut screen_texture_color_texobj,
            //         screen_texture_color_data.ptr,
            //         640,
            //         480,
            //         GX_TF_RGBA8 as u8,
            //         GX_CLAMP as u8,
            //         GX_CLAMP as u8,
            //         GX_FALSE as u8,
            //     );
            //     GX_InitTexObjFilterMode(
            //         &mut screen_texture_color_texobj,
            //         GX_NEAR as u8,
            //         GX_NEAR as u8,
            //     );
            //     GX_LoadTexObj(&mut screen_texture_color_texobj, GX_TEXMAP6 as u8);
            // }

            // Set up texture objects for cluster lightmaps.
            let mut cluster_lightmaps = Vec::new();
            for cluster_index in 0..map_data.lightmap_cluster_table().len() {
                let mut lightmap = Lightmap::new(&map_data, cluster_index);
                lightmap.update(&map_data, cluster_index, 0);
                cluster_lightmaps.push(lightmap);
            }
            GX_InvalidateTexAll();

            // Configure a texture object for the identity map.
            let mut identity_map_data: Vec<u8, _> =
                Vec::with_capacity_in(256 * 256 * 4, GlobalAlign32);
            {
                let mut texels = identity_map_data.as_mut_ptr();
                for coarse_y in 0..64 {
                    for coarse_x in 0..64 {
                        for fine_y in 0..4 {
                            for fine_x in 0..4 {
                                let x = (4 * coarse_x + fine_x) as u8;
                                let _y = (4 * coarse_y + fine_y) as u8;
                                // A
                                (*texels) = 255;
                                texels = texels.offset(1);
                                // R
                                (*texels) = x;
                                texels = texels.offset(1);
                            }
                        }
                        for fine_y in 0..4 {
                            for fine_x in 0..4 {
                                let _x = (4 * coarse_x + fine_x) as u8;
                                let y = (4 * coarse_y + fine_y) as u8;
                                // G
                                (*texels) = y;
                                texels = texels.offset(1);
                                // B
                                (*texels) = 0;
                                texels = texels.offset(1);
                            }
                        }
                    }
                }
                DCFlushRange(identity_map_data.as_mut_ptr() as *mut c_void, 256 * 256 * 4);

                let mut texobj = zeroed::<GXTexObj>();
                GX_InitTexObj(
                    &mut texobj,
                    identity_map_data.as_mut_ptr() as *mut c_void,
                    256,
                    256,
                    GX_TF_RGBA8 as u8,
                    GX_CLAMP as u8,
                    GX_CLAMP as u8,
                    GX_FALSE as u8,
                );
                GX_InitTexObjFilterMode(&mut texobj, GX_NEAR as u8, GX_NEAR as u8);
                GX_LoadTexObj(&mut texobj, GX_TEXMAP7 as u8);
            }

            // Set up texture objects for all other textures.
            let texture_data = map_data.texture_data();
            let map_texobjs: Vec<GXTexObj> = map_data
                .texture_table()
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

            let visibility = Visibility::new(map_data.visibility().as_ptr());

            let mut game_state = GameState {
                pos: guVector {
                    x: -4875.0,
                    y: -1237.0,
                    z: 140.0,
                },
                yaw: core::f32::consts::PI,
                pitch: 0.0,
                inverted_pitch_control: false,
                widescreen: get_widescreen_setting(),
                lightmap_style: 0,
            };

            let mut last_frame_timers = zeroed::<FrameTimers>();
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
                    do_game_logic(&map_data, &mut game_state, &mut cluster_lightmaps);
                });
                let (main_draw_elapsed, view_cluster) = Timer::time_with_result(|| {
                    prepare_main_draw(width, height, &game_state);
                    do_main_draw(
                        &map_data,
                        &game_state,
                        visibility,
                        &map_texobjs,
                        &cluster_lightmaps,
                    )
                });
                let copy_to_texture_elapsed = Timer::time(|| {
                    // do_copy_to_texture(&screen_texture_color_data);
                });
                let debug_draw_elapsed = Timer::time(|| {
                    do_debug_draw(
                        width,
                        height,
                        &game_state,
                        &last_frame_timers,
                        view_cluster,
                        &cluster_lightmaps,
                    );
                });
                let draw_done_elapsed = Timer::time(|| {
                    GX_DrawDone();
                    DO_COPY.store(true, Ordering::Release);
                });
                let idle_elapsed = Timer::time(|| {
                    VIDEO_WaitVSync();
                });

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
    widescreen: bool,
    lightmap_style: usize,
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

        let mut new_lightmap_style = game_state.lightmap_style;
        if (PAD_ButtonsDown(0) & PAD_BUTTON_UP as u16) != 0 {
            new_lightmap_style = new_lightmap_style.wrapping_add(1);
        }
        if (PAD_ButtonsDown(0) & PAD_BUTTON_DOWN as u16) != 0 {
            new_lightmap_style = new_lightmap_style.wrapping_sub(1);
        }
        new_lightmap_style %= 4;

        if game_state.lightmap_style != new_lightmap_style {
            game_state.lightmap_style = new_lightmap_style;
            for (cluster_index, lightmap) in cluster_lightmaps.iter_mut().enumerate() {
                lightmap.update(map_data, cluster_index, game_state.lightmap_style);
            }
        }
    }
}

fn prepare_main_draw(width: u16, height: u16, game_state: &GameState) {
    unsafe {
        load_camera_proj_matrix(width, height, game_state);

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

fn load_camera_proj_matrix(width: u16, height: u16, game_state: &GameState) {
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
    map_texobjs: &[GXTexObj],
    cluster_lightmaps: &[Lightmap],
) -> i16 {
    draw_skybox(game_state, map_texobjs);
    draw_displacements(map_data, game_state, map_texobjs);
    let view_cluster = draw_visible_clusters(
        map_data,
        game_state,
        cluster_lightmaps,
        map_texobjs,
        visibility,
    );
    view_cluster
}

fn draw_visible_clusters<Data: Deref<Target = [u8]>>(
    map_data: &MapData<Data>,
    game_state: &GameState,
    cluster_lightmaps: &[Lightmap],
    map_texobjs: &[GXTexObj],
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

        let draw_cluster = move |cluster, pass| {
            match cluster_lightmaps.get(cluster as usize) {
                Some(lightmap) => {
                    GX_LoadTexObj(lightmap.texobj(), GX_TEXMAP0 as u8);
                }
                None => return,
            }

            let cluster_geometry = cluster_geometry_table[cluster as usize];
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
                        GX_Flush();
                    }
                    BytecodeOp::SetVertexDesc { attr_list_offset } => {
                        panic!();
                        GX_ClearVtxDesc();
                        GX_SetVtxDescv(null_mut());
                    }
                    BytecodeOp::SetBaseTexture { base_texture_id } => {
                        GX_LoadTexObj(
                            &map_texobjs[base_texture_id as usize] as *const GXTexObj
                                as *mut GXTexObj,
                            GX_TEXMAP1 as u8,
                        );
                    }
                    BytecodeOp::SetAuxTexture { aux_texture_id } => {
                        GX_LoadTexObj(
                            &map_texobjs[aux_texture_id as usize] as *const GXTexObj
                                as *mut GXTexObj,
                            GX_TEXMAP2 as u8,
                        );
                    }
                    BytecodeOp::SetEnvTexture { env_texture_id } => {
                        GX_LoadTexObj(
                            &map_texobjs[env_texture_id as usize] as *const GXTexObj
                                as *mut GXTexObj,
                            GX_TEXMAP3 as u8,
                        );
                    }
                    BytecodeOp::SetEnvMapTint { rgb: [r, g, b] } => {
                        GX_SetTevKColor(GX_KCOLOR0 as u8, GXColor { r, g, b, a: 255 });
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
                }
            }
        };

        for pass in 0..18 {
            if pass < 16 {
                match pass & 0x7 {
                    0 => LIGHTMAPPED_SHADER.apply(),
                    1 | 2 => LIGHTMAPPED_ENV_SHADER.apply(),
                    3 => LIGHTMAPPED_ENV_EMAI_SHADER.apply(),
                    4 => LIGHTMAPPED_BAAA_SHADER.apply(),
                    5 | 6 => LIGHTMAPPED_BAAA_ENV_SHADER.apply(),
                    7 => LIGHTMAPPED_BAAA_ENV_EMAI_SHADER.apply(),
                    _ => unreachable!(),
                }
                // } else if pass == 16 {
                //     UNLIT_GENERIC_SHADER.apply();
                // } else if pass == 17 {
                //     WORLD_VERTEX_TRANSITION_SHADER.apply();
            }

            let blend = pass < 16 && (pass & 8) == 8;
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
        GX_DrawDone();

        view_cluster
    }
}

fn draw_skybox(game_state: &GameState, map_texobjs: &[GXTexObj]) {
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
            &map_texobjs[0] as *const GXTexObj as *mut GXTexObj,
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
            &map_texobjs[1] as *const GXTexObj as *mut GXTexObj,
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
            &map_texobjs[2] as *const GXTexObj as *mut GXTexObj,
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
            &map_texobjs[3] as *const GXTexObj as *mut GXTexObj,
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
            &map_texobjs[4] as *const GXTexObj as *mut GXTexObj,
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
    map_texobjs: &[GXTexObj],
) {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_CLR0 as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX1 as u8, GX_INDEX16 as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_CLR0, GX_CLR_RGB, GX_RGB8, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_F32, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX1, GX_TEX_ST, GX_F32, 0);
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
            GX_VA_TEX0,
            map_data.displacement_texture_coordinate_data().as_ptr() as *mut _,
            8,
        );
        GX_SetArray(
            GX_VA_TEX1,
            map_data.displacement_texture_coordinate_data().as_ptr() as *mut _,
            8,
        );
        GX_InvVtxCache();

        load_camera_view_matrix(game_state);

        GX_SetBlendMode(GX_BM_NONE as u8, 0, 0, 0);
        GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);

        let displacement_byte_code = map_data.displacement_byte_code();
        let displacement_display_lists = map_data.displacement_display_lists();

        for (mode, entry) in map_data.displacement_table().iter().enumerate() {
            match mode {
                0 => UNLIT_GENERIC_SHADER.apply(),
                1 => WORLD_VERTEX_TRANSITION_SHADER.apply(),
                _ => unreachable!(),
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
                    BytecodeOp::SetBaseTexture { base_texture_id } => {
                        GX_LoadTexObj(
                            &map_texobjs[base_texture_id as usize] as *const GXTexObj
                                as *mut GXTexObj,
                            GX_TEXMAP1 as u8,
                        );
                    }
                    BytecodeOp::SetAuxTexture { aux_texture_id } => {
                        GX_LoadTexObj(
                            &map_texobjs[aux_texture_id as usize] as *const GXTexObj
                                as *mut GXTexObj,
                            GX_TEXMAP2 as u8,
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }

        GX_Flush();
    }
}

fn _do_copy_to_texture(screen_texture_color_data: &Memalign<32>) {
    unsafe {
        // Copy the color buffer to a texture in main memory.
        GX_SetTexCopySrc(0, 0, 640, 480); // TODO: Use the current mode.

        DCInvalidateRange(
            screen_texture_color_data.as_void_ptr_mut(),
            screen_texture_color_data.size() as u32,
        );
        GX_SetTexCopyDst(640, 480, GX_TF_RGBA8, GX_FALSE as u8);
        GX_CopyTex(screen_texture_color_data.as_void_ptr_mut(), GX_FALSE as u8);

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

fn do_debug_draw(
    width: u16,
    height: u16,
    game_state: &GameState,
    last_frame_timers: &FrameTimers,
    view_cluster: i16,
    cluster_lightmaps: &[Lightmap],
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
    }
}

fn init_for_console() -> (*mut GXRModeObj, u16, u16) {
    unsafe {
        VIDEO_Init();
        PAD_Init();

        // Configure the preferred video mode.
        let rmode = VIDEO_GetPreferredMode(null_mut());
        VIDEO_Configure(rmode);

        // Allocate an external frame buffer, set up a vblank callback to swap buffers, and wait two
        // frames (for hardware to warm up?).
        let mut xfb = XFB.load(Ordering::Acquire);
        if xfb.is_null() {
            xfb = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
            XFB.store(xfb, Ordering::Release);
        }
        VIDEO_ClearFrameBuffer(rmode, xfb, 0x80808080);
        VIDEO_SetNextFramebuffer(xfb);
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
        drop(VIDEO_SetPostRetraceCallback(Some(copy_to_xfb)));

        // Allocate a FIFO for sending commands to the GPU.
        let gp_fifo = GP_FIFO.load(Ordering::Acquire);
        if gp_fifo.is_null() {
            const FIFO_SIZE: usize = 256 * 1024;
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
        GX_SetViewport(
            0.0,
            0.0,
            rmode.fbWidth as f32,
            rmode.efbHeight as f32,
            0.0,
            1.0,
        );
        GX_SetDispCopyYScale(rmode.xfbHeight as f32 / rmode.efbHeight as f32);
        GX_SetScissor(0, 0, rmode.fbWidth as u32, rmode.efbHeight as u32);
        GX_SetDispCopySrc(0, 0, rmode.fbWidth, rmode.efbHeight);
        GX_SetDispCopyDst(rmode.fbWidth, rmode.xfbHeight);
        GX_SetCopyFilter(
            rmode.aa,
            rmode.sample_pattern.as_ptr() as *mut [u8; 2],
            GX_TRUE as u8,
            rmode.vfilter.as_ptr() as *mut u8,
        );
        GX_SetFieldMode(
            rmode.field_rendering,
            if rmode.viHeight == 2 * rmode.xfbHeight {
                GX_ENABLE
            } else {
                GX_DISABLE
            } as u8,
        );
        GX_SetPixelFmt(
            if rmode.aa != 0 {
                GX_PF_RGB565_Z16
            } else {
                GX_PF_RGB8_Z24
            } as u8,
            GX_ZC_LINEAR as u8,
        );

        GX_SetCullMode(GX_CULL_BACK as u8);
        GX_SetDispCopyGamma(GX_GM_1_0 as u8);
        GX_CopyDisp(XFB.load(Ordering::Acquire), GX_TRUE as u8);
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

extern "C" fn copy_to_xfb(_count: u32) {
    if DO_COPY
        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        unsafe {
            GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);
            GX_SetColorUpdate(GX_TRUE as u8);
            GX_CopyDisp(XFB.load(Ordering::Acquire), GX_TRUE as u8);
            GX_Flush();
        }
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
