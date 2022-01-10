use alloc::vec::Vec;
use fully_occupied::FullyOccupied;
use inception_render_common::bytecode::{BytecodeOp, BytecodeReader};
use ogc_sys::guVector;

#[derive(Clone, Copy)]
#[repr(align(32))]
pub struct Align32Bytes(pub [u8; 32]);

unsafe impl FullyOccupied for Align32Bytes {}

pub struct MapData {
    pub position_data: Vec<u8>,
    pub normal_data: Vec<u8>,
    pub texture_coord_data: Vec<u8>,
    pub cluster_geometry_table: Vec<ClusterGeometry>,
    pub cluster_geometry_byte_code: Vec<u32>,
    pub cluster_geometry_display_lists: Vec<Align32Bytes>,

    pub bsp_nodes: Vec<BspNode>,
    pub bsp_leaves: Vec<BspLeaf>,
    pub visibility: Vec<u8>,

    pub texture_table: Vec<TextureTableEntry>,
    pub texture_data: Vec<Align32Bytes>,

    pub lightmap_cluster_table: Vec<LightmapClusterTableEntry>,
    pub lightmap_patch_table: Vec<LightmapPatchTableEntry>,
    pub lightmap_data: Vec<u8>,

    pub displacement_position_data: Vec<u8>,
    pub displacement_vertex_color_data: Vec<u8>,
    pub displacement_texture_coordinate_data: Vec<u8>,
    pub displacement_table: Vec<DisplacementTableEntry>,
    pub displacement_byte_code: Vec<u32>,
    pub displacement_display_lists: Vec<Align32Bytes>,
}

impl MapData {
    pub fn traverse_bsp(&self, pos: &guVector) -> &BspLeaf {
        let mut node = &self.bsp_nodes[0];
        loop {
            let d = node.plane[0] * pos.x + node.plane[1] * pos.y + node.plane[2] * pos.z;
            let child = node.children[if d > node.plane[3] { 0 } else { 1 }];
            if child < 0 {
                let leaf_index = child.wrapping_neg().wrapping_sub(1) as usize;
                return &self.bsp_leaves[leaf_index];
            } else {
                node = &self.bsp_nodes[child as usize];
            }
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ClusterGeometry {
    byte_code_index_ranges: [[usize; 2]; 18],
}

unsafe impl FullyOccupied for ClusterGeometry {}

impl ClusterGeometry {
    pub fn iter_display_lists<'a>(
        &'a self,
        map_data: &'a MapData,
        pass: usize,
    ) -> impl Iterator<Item = BytecodeOp> + 'a {
        BytecodeReader::new(
            &map_data.cluster_geometry_byte_code
                [self.byte_code_index_ranges[pass][0]..self.byte_code_index_ranges[pass][1]],
        )
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BspNode {
    pub plane: [f32; 4],
    pub children: [i32; 2],
}

unsafe impl FullyOccupied for BspNode {}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BspLeaf {
    pub cluster: i16,
}

unsafe impl FullyOccupied for BspLeaf {}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TextureTableEntry {
    pub width: u16,
    pub height: u16,
    pub mip_count: u8,
    pub flags: u8,
    /// One of the GX_TF_* enumerated values.
    pub format: u8,
    pub _padding: u8,
    pub start_index: usize,
    pub end_index: usize,
}

pub const TEXTURE_FLAG_CLAMP_S: u8 = 0x01;
pub const TEXTURE_FLAG_CLAMP_T: u8 = 0x02;

unsafe impl FullyOccupied for TextureTableEntry {}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct LightmapClusterTableEntry {
    pub width: u16,
    pub height: u16,
    pub patch_table_start_index: usize,
    pub patch_table_end_index: usize,
}

unsafe impl FullyOccupied for LightmapClusterTableEntry {}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct LightmapPatchTableEntry {
    pub sub_block_x: u8,
    pub sub_block_y: u8,
    pub sub_blocks_wide: u8,
    pub sub_blocks_high: u8,
    pub style_count: u8,
    pub _padding1: u8,
    pub _padding2: u16,
    pub data_start_offset: usize,
    pub data_end_offset: usize,
}

unsafe impl FullyOccupied for LightmapPatchTableEntry {}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct DisplacementTableEntry {
    pub byte_code_start_index: usize,
    pub byte_code_end_index: usize,
}

unsafe impl FullyOccupied for DisplacementTableEntry {}
