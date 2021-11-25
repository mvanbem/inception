#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(default_alloc_error_handler)]
#![feature(start)]

extern crate alloc;

use core::ffi::c_void;
use core::mem::zeroed;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use alloc::vec::Vec;
use fully_occupied::{extract_slice, FullyOccupied};
use ogc_sys::*;

use crate::shaders::flat_vertex_color::FLAT_VERTEX_COLOR_SHADER;
use crate::shaders::lightmapped::LIGHTMAPPED_SHADER;
use crate::visibility::{ClusterIndex, Visibility};

#[macro_use]
mod include_bytes_align;

mod gx;
mod shader;
mod shaders;
mod visibility;

static LIGHTMAP_DATA: &[u8] = include_bytes_align!(32, "../../../build/lightmap_cmpr.tpl");
static POSITION_DATA: &[u8] = include_bytes_align!(32, "../../../build/position_data.dat");
static LIGHTMAP_COORD_DATA: &[u8] =
    include_bytes_align!(32, "../../../build/lightmap_coord_data.dat");
static TEXTURE_COORD_DATA: &[u8] =
    include_bytes_align!(32, "../../../build/texture_coord_data.dat");
static DISPLAY_LISTS_DATA: &[u8] = include_bytes_align!(32, "../../../build/display_lists.dat");
static BSP_NODE_DATA: &[u8] = include_bytes_align!(4, "../../../build/bsp_nodes.dat");
static BSP_LEAF_DATA: &[u8] = include_bytes_align!(2, "../../../build/bsp_leaves.dat");
static VISIBILITY_DATA: &[u8] = include_bytes_align_as!(u32, "../../../build/vis.dat");
static TEXTURE_TABLE_DATA: &[u8] = include_bytes_align_as!(u32, "../../../build/texture_table.dat");
static TEXTURE_DATA: &[u8] = include_bytes_align!(32, "../../../build/texture_data.dat");

static XFB: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static DO_COPY: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct FirstDisplayListEntry {
    second_index_start_offset: usize,
    second_index_end_offset: usize,
}

unsafe impl FullyOccupied for FirstDisplayListEntry {}

#[repr(C)]
struct SecondDisplayListEntry {
    texture_index: usize,
    display_list_start_offset: usize,
    display_list_end_offset: usize,
}

unsafe impl FullyOccupied for SecondDisplayListEntry {}

struct TextureDisplayList {
    texture_index: usize,
    display_list: &'static [u8],
}

fn iter_display_lists_for_cluster(cluster: u16) -> impl Iterator<Item = TextureDisplayList> {
    let first_entry = &extract_slice::<FirstDisplayListEntry>(DISPLAY_LISTS_DATA)[cluster as usize];
    extract_slice::<SecondDisplayListEntry>(
        &DISPLAY_LISTS_DATA
            [first_entry.second_index_start_offset..first_entry.second_index_end_offset],
    )
    .iter()
    .map(|second_entry| TextureDisplayList {
        texture_index: second_entry.texture_index,
        display_list: &DISPLAY_LISTS_DATA
            [second_entry.display_list_start_offset..second_entry.display_list_end_offset],
    })
}

#[repr(C)]
struct TextureTableEntry {
    width: u16,
    height: u16,
    mip_count: u8,
    _padding1: u8,
    _padding2: u16,
    start_offset: usize,
    end_offset: usize,
}

unsafe impl FullyOccupied for TextureTableEntry {}

fn texture_table() -> &'static [TextureTableEntry] {
    extract_slice::<TextureTableEntry>(TEXTURE_TABLE_DATA)
}

#[repr(C)]
struct BspNode {
    plane: [f32; 4],
    children: [i32; 2],
}

#[repr(C)]
struct BspLeaf {
    cluster: i16,
}

unsafe fn traverse_bsp(pos: &guVector) -> *const BspLeaf {
    unsafe {
        let bsp_nodes = BSP_NODE_DATA.as_ptr() as *const BspNode;
        let bsp_leaves = BSP_LEAF_DATA.as_ptr() as *const BspLeaf;

        let mut node = bsp_nodes;
        loop {
            let d = (*node).plane[0] * pos.x + (*node).plane[1] * pos.y + (*node).plane[2] * pos.z;
            let child = (*node).children[if d > (*node).plane[3] { 0 } else { 1 }];
            if child < 0 {
                let leaf_index = child.wrapping_neg().wrapping_sub(1) as u32;
                return bsp_leaves.offset(leaf_index as isize);
            } else {
                node = bsp_nodes.offset(child as isize);
            }
        }
    }
}

#[cfg(feature = "wii")]
fn get_widescreen_setting() -> bool {
    unsafe { CONF_GetAspectRatio() != 0 }
}

#[cfg(not(feature = "wii"))]
fn get_widescreen_setting() -> bool {
    false
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        let (width, height) = init_hardware();

        // Configure a texture object for the lightmap.
        let mut lightmap_tpl = zeroed::<TPLFile>();
        assert_eq!(
            TPL_OpenTPLFromMemory(
                &mut lightmap_tpl,
                LIGHTMAP_DATA.as_ptr() as *mut c_void,
                LIGHTMAP_DATA.len() as u32
            ),
            1,
        );
        let mut lightmap_texobj = zeroed::<GXTexObj>();
        assert_eq!(
            TPL_GetTexture(&mut lightmap_tpl, 0, &mut lightmap_texobj),
            0,
        );
        GX_LoadTexObj(&mut lightmap_texobj, GX_TEXMAP0 as u8);

        // Set up texture objects for all other textures.
        let base_map_texobjs: Vec<GXTexObj> = texture_table()
            .iter()
            .map(|entry| {
                let mut texobj = zeroed::<GXTexObj>();
                GX_InitTexObj(
                    &mut texobj,
                    TEXTURE_DATA[entry.start_offset..entry.end_offset].as_ptr() as *mut c_void,
                    entry.width,
                    entry.height,
                    GX_TF_CMPR as u8,
                    GX_REPEAT as u8,
                    GX_REPEAT as u8,
                    if entry.mip_count > 0 {
                        GX_TRUE
                    } else {
                        GX_FALSE
                    } as u8,
                );
                GX_InitTexObjLOD(
                    &mut texobj,
                    if entry.mip_count > 0 {
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
                    GX_ANISO_4 as u8,
                );
                texobj
            })
            .collect();

        let visibility = Visibility::new(VISIBILITY_DATA.as_ptr());

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
        };

        let mut last_frame_timers = zeroed::<FrameTimers>();
        let mut proj = zeroed::<Mtx44>();
        let mut view = zeroed::<Mtx>();
        loop {
            let game_logic_elapsed = Timer::time(|| {
                do_game_logic(&mut game_state);
            });
            let draw_setup_elapsed = Timer::time(|| {
                prepare_main_draw(width, height, &game_state, &mut proj, &mut view);
            });
            let (draw_calls_elapsed, view_cluster) = Timer::time_with_result(|| {
                do_main_draw(&game_state.pos, visibility, &base_map_texobjs)
            });
            let debug_draw_elapsed = Timer::time(|| {
                do_debug_draw(
                    height,
                    &last_frame_timers,
                    view_cluster,
                    &mut proj,
                    &mut view,
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
                draw_setup: draw_setup_elapsed,
                draw_calls: draw_calls_elapsed,
                debug_draw: debug_draw_elapsed,
                draw_done: draw_done_elapsed,
                idle: idle_elapsed,
            };
        }
    }
}

struct GameState {
    pos: guVector,
    yaw: f32,
    pitch: f32,
    inverted_pitch_control: bool,
    widescreen: bool,
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

fn do_game_logic(game_state: &mut GameState) {
    unsafe {
        PAD_ScanPads();

        if (PAD_ButtonsDown(0) & PAD_BUTTON_START as u16) != 0 {
            libc::exit(0);
        }
        if (PAD_ButtonsDown(0) & PAD_TRIGGER_Z as u16) != 0 {
            game_state.inverted_pitch_control ^= true;
        }

        let right = [libm::sinf(game_state.yaw), -libm::cosf(game_state.yaw), 0.0];
        let forward = [libm::cosf(game_state.yaw), libm::sinf(game_state.yaw), 0.0];
        let speed = if PAD_TriggerR(0) >= 128 { 100.0 } else { 10.0 };
        let angspeed = 0.05;
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
        if (PAD_ButtonsHeld(0) & PAD_BUTTON_UP as u16) != 0 {
            game_state.pos.z += speed;
        }
        if (PAD_ButtonsHeld(0) & PAD_BUTTON_DOWN as u16) != 0 {
            game_state.pos.z -= speed;
        }

        game_state.yaw -= angspeed * cx;
        game_state.pitch = (game_state.pitch + angspeed * cy).clamp(
            -89.0 / 180.0 * core::f32::consts::PI,
            89.0 / 180.0 * core::f32::consts::PI,
        );
    }
}

fn prepare_main_draw(
    width: u16,
    height: u16,
    game_state: &GameState,
    proj: &mut Mtx44,
    view: &mut Mtx,
) {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_INDEX16 as u8);
        GX_SetVtxDesc(GX_VA_TEX1 as u8, GX_INDEX16 as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_F32, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX1, GX_TEX_ST, GX_F32, 0);
        GX_SetArray(GX_VA_POS, POSITION_DATA.as_ptr() as *mut _, 12);
        GX_SetArray(GX_VA_TEX0, LIGHTMAP_COORD_DATA.as_ptr() as *mut _, 8);
        GX_SetArray(GX_VA_TEX1, TEXTURE_COORD_DATA.as_ptr() as *mut _, 8);
        GX_InvVtxCache();

        guPerspective(
            proj.as_mut_ptr(),
            90.0,
            width as f32 / height as f32 * game_state.widescreen_factor(),
            1.0,
            5000.0,
        );
        GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_PERSPECTIVE as u8);

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
            } as *mut guVector,
            &mut guVector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            } as *mut guVector,
            &mut guVector {
                x: game_state.pos.x + 1.0,
                y: game_state.pos.y,
                z: game_state.pos.z,
            } as *mut guVector,
        );
        c_guMtxRotRad(yaw_rotation.as_mut_ptr(), b'y', -game_state.yaw);
        c_guMtxRotRad(pitch_rotation.as_mut_ptr(), b'x', -game_state.pitch);
        c_guMtxConcat(
            yaw_rotation.as_mut_ptr(),
            look_at.as_mut_ptr(),
            tmp.as_mut_ptr(),
        );
        c_guMtxConcat(
            pitch_rotation.as_mut_ptr(),
            tmp.as_mut_ptr(),
            view.as_mut_ptr(),
        );
        GX_LoadPosMtxImm(view.as_mut_ptr(), GX_PNMTX0);
    }
}

fn do_main_draw(pos: &guVector, visibility: Visibility, base_map_texobjs: &[GXTexObj]) -> i16 {
    unsafe {
        LIGHTMAPPED_SHADER.apply();

        GX_SetZMode(GX_TRUE as u8, GX_LEQUAL as u8, GX_TRUE as u8);
        GX_SetColorUpdate(GX_TRUE as u8);

        let view_leaf = traverse_bsp(pos);
        let view_cluster = (*view_leaf).cluster;
        if view_cluster != -1 {
            for cluster in visibility
                .get_cluster(ClusterIndex(view_cluster as usize))
                .iter_visible_clusters()
                .map(|cluster| cluster.0 as u16)
            {
                for entry in iter_display_lists_for_cluster(cluster) {
                    GX_LoadTexObj(
                        &base_map_texobjs[entry.texture_index] as *const GXTexObj as *mut GXTexObj,
                        GX_TEXMAP1 as u8,
                    );
                    GX_CallDispList(
                        entry.display_list.as_ptr() as *mut c_void,
                        entry.display_list.len() as u32,
                    );
                    GX_Flush();
                }
            }
        } else {
            for cluster in 0..visibility.num_clusters() as u16 {
                for entry in iter_display_lists_for_cluster(cluster) {
                    GX_LoadTexObj(
                        &base_map_texobjs[entry.texture_index] as *const GXTexObj as *mut GXTexObj,
                        GX_TEXMAP1 as u8,
                    );
                    GX_CallDispList(
                        entry.display_list.as_ptr() as *mut c_void,
                        entry.display_list.len() as u32,
                    );
                    GX_Flush();
                }
            }
        }

        view_cluster
    }
}

fn do_debug_draw(
    height: u16,
    last_frame_timers: &FrameTimers,
    view_cluster: i16,
    proj: &mut Mtx44,
    view: &mut Mtx,
) {
    unsafe {
        GX_ClearVtxDesc();
        GX_SetVtxDesc(GX_VA_POS as u8, GX_DIRECT as u8);
        GX_SetVtxDesc(GX_VA_CLR0 as u8, GX_DIRECT as u8);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XY, GX_U16, 0);
        GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_CLR0, GX_CLR_RGB, GX_RGB8, 0);
        GX_InvVtxCache();

        GX_SetNumTexGens(0);

        FLAT_VERTEX_COLOR_SHADER.apply();

        GX_SetCullMode(GX_CULL_NONE as u8);
        GX_SetZMode(GX_FALSE as u8, GX_ALWAYS as u8, GX_FALSE as u8);
        GX_SetColorUpdate(GX_TRUE as u8);

        guOrtho(proj.as_mut_ptr(), 0.0, 480.0, 0.0, 640.0, -1.0, 1.0);
        GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_ORTHOGRAPHIC as u8);

        c_guMtxIdentity(view.as_mut_ptr());
        GX_LoadPosMtxImm(view.as_mut_ptr(), GX_PNMTX0);

        let to_y = height - 16;
        let from_y = to_y - 16;
        let emit_debug_quad = |from_x, to_x, max_x, r, g, b| {
            let from_x = (from_x as f32 * 640.0 / max_x as f32) as u16;
            let to_x = (to_x as f32 * 640.0 / max_x as f32) as u16;

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
        let x2 = x1 + last_frame_timers.draw_setup;
        let x3 = x2 + last_frame_timers.draw_calls;
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
    }
}

fn init_hardware() -> (u16, u16) {
    unsafe {
        // Initialize the libogc subsystems that we're using. GX is initialized further down because
        // it needs an allocated FIFO.
        VIDEO_Init();
        PAD_Init();

        // Configure the preferred video mode.
        let rmode = VIDEO_GetPreferredMode(null_mut());
        VIDEO_Configure(rmode);

        // Allocate an external frame buffer, set up a vblank callback to swap buffers, and wait two
        // frames (for hardware to warm up?).
        let xfb = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
        XFB.store(xfb, Ordering::Release);
        VIDEO_ClearFrameBuffer(rmode, xfb, 0x80808080);
        VIDEO_SetNextFramebuffer(xfb);
        drop(VIDEO_SetPostRetraceCallback(Some(copy_to_xfb)));
        VIDEO_SetBlack(false);
        VIDEO_Flush();
        VIDEO_WaitVSync();
        if ((*rmode).viTVMode & VI_NON_INTERLACE) != 0 {
            VIDEO_WaitVSync();
        }

        // Allocate a FIFO for sending commands to the GPU.
        const FIFO_SIZE: usize = 256 * 1024;
        let gp_fifo = MEM_K0_TO_K1(libc::memalign(32, FIFO_SIZE));
        libc::memset(gp_fifo, 0, FIFO_SIZE);
        GX_Init(gp_fifo, FIFO_SIZE as u32);

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
            (*rmode).fbWidth as f32,
            (*rmode).efbHeight as f32,
            0.0,
            1.0,
        );
        GX_SetDispCopyYScale((*rmode).xfbHeight as f32 / (*rmode).efbHeight as f32);
        GX_SetScissor(0, 0, (*rmode).fbWidth as u32, (*rmode).efbHeight as u32);
        GX_SetDispCopySrc(0, 0, (*rmode).fbWidth, (*rmode).efbHeight);
        GX_SetDispCopyDst((*rmode).fbWidth, (*rmode).xfbHeight);
        GX_SetCopyFilter(
            (*rmode).aa,
            (*rmode).sample_pattern.as_mut_ptr(),
            GX_TRUE as u8,
            (*rmode).vfilter.as_mut_ptr(),
        );
        GX_SetFieldMode(
            (*rmode).field_rendering,
            if (*rmode).viHeight == 2 * (*rmode).xfbHeight {
                GX_ENABLE
            } else {
                GX_DISABLE
            } as u8,
        );
        GX_SetPixelFmt(
            if (*rmode).aa != 0 {
                GX_PF_RGB565_Z16
            } else {
                GX_PF_RGB8_Z24
            } as u8,
            GX_ZC_LINEAR as u8,
        );

        GX_SetCullMode(GX_CULL_NONE as u8);
        GX_CopyDisp(xfb, GX_TRUE as u8);
        GX_SetDispCopyGamma(GX_GM_1_0 as u8);

        ((*rmode).fbWidth, (*rmode).efbHeight)
    }
}

struct FrameTimers {
    game_logic: u32,
    draw_setup: u32,
    draw_calls: u32,
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
