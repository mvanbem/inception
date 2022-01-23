use core::ops::Deref;
use core::slice;
#[cfg(feature = "std")]
use std::borrow::Cow;
#[cfg(feature = "std")]
use std::io::{self, Seek, Write};

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "std")]
use byteorder::{BigEndian, WriteBytesExt};
#[cfg(feature = "std")]
use relocation::{PointerFormat, RelocationWriter};

use crate::bytecode::{BytecodeOp, BytecodeReader};

#[cfg(feature = "std")]
pub trait WriteTo<W: Seek + Write> {
    fn write_to(&self, w: &mut W) -> io::Result<()>;
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for u32 {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(*self)
    }
}

#[cfg(feature = "std")]
impl<W: Seek + Write, T: WriteTo<W>> WriteTo<W> for [T] {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        for item in self {
            item.write_to(w)?;
        }
        Ok(())
    }
}

// #[derive(Clone, Copy, Pod, Zeroable)]
// #[repr(C, align(32))]
// pub struct Align32Bytes(pub [u8; 32]);
//
// #[cfg(feature = "std")]
// impl<W: Seek + Write> WriteTo<W> for Align32Bytes {
//     fn write_to(&self, w: &mut W) -> io::Result<()> {
//         w.write_all(&self.0)
//     }
// }

// TODO: Complete these aspirational new formats.
//
// #[derive(Clone, Copy, Pod, Zeroable)]
// #[repr(C)]
// pub struct ClusterGeometry {
//     pub opaque_batches_offset: u32,
//     pub translucent_batches_offset: u32,
//     pub opaque_batches_count: u8,
//     pub translucent_batches_count: u8,
//     pub padding: [u8; 2],
// }
//
// #[derive(Clone, Copy, Pod, Zeroable)]
// #[repr(C)]
// pub struct PackedBatch {
//     pub shader_id: u8,
//     pub padding: [u8; 3],
//     pub bytecode_start_offset: u32,
//     pub bytecode_end_offset: u32,
// }

pub struct OwnedMapData {
    pub position_data: Vec<u8>,
    pub normal_data: Vec<u8>,
    pub texture_coord_data: Vec<u8>,
    pub cluster_geometry_table: Vec<ClusterGeometryTableEntry>,
    pub cluster_geometry_byte_code: Vec<u32>,
    pub cluster_geometry_display_lists: Vec<u8>,

    pub bsp_nodes: Vec<BspNode>,
    pub bsp_leaves: Vec<BspLeaf>,
    pub visibility: Vec<u8>,

    pub texture_table: Vec<TextureTableEntry>,
    pub texture_data: Vec<u8>,

    pub lightmap_cluster_table: Vec<ClusterLightmapTableEntry>,
    pub lightmap_displacement_table: Vec<DisplacementLightmapTableEntry>,
    pub lightmap_patch_table: Vec<LightmapPatchTableEntry>,
    pub lightmap_data: Vec<u8>,

    pub displacement_position_data: Vec<u8>,
    pub displacement_vertex_color_data: Vec<u8>,
    pub displacement_texture_coordinate_data: Vec<u8>,
    pub displacement_table: Vec<DisplacementTableEntry>,
    pub displacement_byte_code: Vec<u32>,
    pub displacement_display_lists: Vec<u8>,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for OwnedMapData {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        let mut w = RelocationWriter::new(w);

        // Write the header, which is a list of offsets to the sections.

        fn write_slice_header<T, W: Seek + Write>(
            w: &mut RelocationWriter<W>,
            symbol: &'static str,
            slice: &[T],
        ) -> io::Result<()> {
            w.write_pointer(PointerFormat::BigEndianU32, Cow::Borrowed(symbol))?;
            w.write_u32::<BigEndian>(u32::try_from(slice.len()).unwrap())?;
            Ok(())
        }

        macro_rules! write_slice_header {
            ($name:ident) => {
                write_slice_header(&mut w, stringify!($name), &self.$name)?;
            };
        }

        write_slice_header!(position_data);
        write_slice_header!(normal_data);
        write_slice_header!(texture_coord_data);
        write_slice_header!(cluster_geometry_table);
        write_slice_header!(cluster_geometry_byte_code);
        write_slice_header!(cluster_geometry_display_lists);
        write_slice_header!(bsp_nodes);
        write_slice_header!(bsp_leaves);
        write_slice_header!(visibility);
        write_slice_header!(texture_table);
        write_slice_header!(texture_data);
        write_slice_header!(lightmap_cluster_table);
        write_slice_header!(lightmap_displacement_table);
        write_slice_header!(lightmap_patch_table);
        write_slice_header!(lightmap_data);
        write_slice_header!(displacement_position_data);
        write_slice_header!(displacement_vertex_color_data);
        write_slice_header!(displacement_texture_coordinate_data);
        write_slice_header!(displacement_table);
        write_slice_header!(displacement_byte_code);
        write_slice_header!(displacement_display_lists);

        // Write each section.

        fn write_slice_data<T: WriteTo<W>, W: Seek + Write>(
            w: &mut RelocationWriter<W>,
            symbol: &'static str,
            slice: &[T],
        ) -> io::Result<()> {
            while w.stream_position()? % std::mem::align_of::<T>() as u64 != 0 {
                w.write_u8(0)?;
            }
            w.define_symbol_here(Cow::Borrowed(symbol))?;
            slice.write_to(&mut *w)?;
            Ok(())
        }

        fn write_slice_bytes<W: Seek + Write>(
            w: &mut RelocationWriter<W>,
            symbol: &'static str,
            slice: &[u8],
            align: u64,
        ) -> io::Result<()> {
            while w.stream_position()? % align != 0 {
                w.write_u8(0)?;
            }
            w.define_symbol_here(Cow::Borrowed(symbol))?;
            w.write_all(slice)?;
            Ok(())
        }

        macro_rules! write_slice_data {
            ($name:ident) => {
                write_slice_data(&mut w, stringify!($name), &self.$name)?;
            };
        }

        macro_rules! write_slice_bytes {
            ($name:ident $(,)?) => {
                write_slice_bytes(&mut w, stringify!($name), &self.$name, 1)?;
            };
            ($name:ident, $align:literal $(,)?) => {
                write_slice_bytes(&mut w, stringify!($name), &self.$name, $align)?;
            };
        }

        write_slice_bytes!(position_data, 1);
        write_slice_bytes!(normal_data, 1);
        write_slice_bytes!(texture_coord_data, 1);
        write_slice_data!(cluster_geometry_table);
        write_slice_data!(cluster_geometry_byte_code);
        write_slice_bytes!(cluster_geometry_display_lists, 32);
        write_slice_data!(bsp_nodes);
        write_slice_data!(bsp_leaves);
        write_slice_bytes!(visibility);
        write_slice_data!(texture_table);
        write_slice_bytes!(texture_data, 32);
        write_slice_data!(lightmap_cluster_table);
        write_slice_data!(lightmap_displacement_table);
        write_slice_data!(lightmap_patch_table);
        write_slice_bytes!(lightmap_data);
        write_slice_bytes!(displacement_position_data);
        write_slice_bytes!(displacement_vertex_color_data);
        write_slice_bytes!(displacement_texture_coordinate_data);
        write_slice_data!(displacement_table);
        write_slice_data!(displacement_byte_code);
        write_slice_bytes!(displacement_display_lists, 32);

        w.finish()?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct PackedMapData {
    position_data_offset: usize,
    position_data_len: usize,
    normal_data_offset: usize,
    normal_data_len: usize,
    texture_coord_data_offset: usize,
    texture_coord_data_len: usize,
    cluster_geometry_table_offset: usize,
    cluster_geometry_table_len: usize,
    cluster_geometry_byte_code_offset: usize,
    cluster_geometry_byte_code_len: usize,
    cluster_geometry_display_lists_offset: usize,
    cluster_geometry_display_lists_len: usize,

    bsp_nodes_offset: usize,
    bsp_nodes_len: usize,
    bsp_leaves_offset: usize,
    bsp_leaves_len: usize,
    visibility_offset: usize,
    visibility_len: usize,

    texture_table_offset: usize,
    texture_table_len: usize,
    texture_data_offset: usize,
    texture_data_len: usize,

    lightmap_cluster_table_offset: usize,
    lightmap_cluster_table_len: usize,
    lightmap_displacement_table_offset: usize,
    lightmap_displacement_table_len: usize,
    lightmap_patch_table_offset: usize,
    lightmap_patch_table_len: usize,
    lightmap_data_offset: usize,
    lightmap_data_len: usize,

    displacement_position_data_offset: usize,
    displacement_position_data_len: usize,
    displacement_vertex_color_data_offset: usize,
    displacement_vertex_color_data_len: usize,
    displacement_texture_coordinate_data_offset: usize,
    displacement_texture_coordinate_data_len: usize,
    displacement_table_offset: usize,
    displacement_table_len: usize,
    displacement_byte_code_offset: usize,
    displacement_byte_code_len: usize,
    displacement_display_lists_offset: usize,
    displacement_display_lists_len: usize,
}

pub struct MapData<Data> {
    data: Data,
}

impl<Data: Deref<Target = [u8]>> MapData<Data> {
    // # Safety
    //
    // The data must encode a PackedMapData struct followed by section data. Every (offset, len)
    // pair must be contained in the section data and be properly aligned. The data must be 32-byte
    // aligned.
    pub unsafe fn new(data: Data) -> Self {
        Self { data }
    }

    fn packed(&self) -> &PackedMapData {
        unsafe { &*(self.data.as_ptr() as *const PackedMapData) }
    }

    unsafe fn cast_slice<T: Pod>(&self, offset: usize, len: usize) -> &[T] {
        let data = (self.data.as_ptr() as usize + offset) as *const T;
        slice::from_raw_parts(data, len)
    }

    pub fn position_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.position_data_offset, packed.position_data_len) }
    }

    pub fn normal_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.normal_data_offset, packed.normal_data_len) }
    }

    pub fn texture_coord_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.texture_coord_data_offset,
                packed.texture_coord_data_len,
            )
        }
    }

    pub fn cluster_geometry_table(&self) -> &[ClusterGeometryTableEntry] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.cluster_geometry_table_offset,
                packed.cluster_geometry_table_len,
            )
        }
    }

    pub fn cluster_geometry_byte_code(&self) -> &[u32] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.cluster_geometry_byte_code_offset,
                packed.cluster_geometry_byte_code_len,
            )
        }
    }

    pub fn cluster_geometry_display_lists(&self) -> &[u8] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.cluster_geometry_display_lists_offset,
                packed.cluster_geometry_display_lists_len,
            )
        }
    }

    pub fn bsp_nodes(&self) -> &[BspNode] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.bsp_nodes_offset, packed.bsp_nodes_len) }
    }

    pub fn bsp_leaves(&self) -> &[BspLeaf] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.bsp_leaves_offset, packed.bsp_leaves_len) }
    }

    pub fn traverse_bsp(&self, pos: &[f32; 3]) -> &BspLeaf {
        let bsp_nodes = self.bsp_nodes();
        let bsp_leaves = self.bsp_leaves();
        let mut node = &bsp_nodes[0];
        loop {
            let d = node.plane[0] * pos[0] + node.plane[1] * pos[1] + node.plane[2] * pos[2];
            let child = node.children[if d > node.plane[3] { 0 } else { 1 }];
            if child < 0 {
                let leaf_index = child.wrapping_neg().wrapping_sub(1) as usize;
                return &bsp_leaves[leaf_index];
            } else {
                node = &bsp_nodes[child as usize];
            }
        }
    }

    pub fn visibility(&self) -> &[u8] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.visibility_offset, packed.visibility_len) }
    }

    pub fn texture_table(&self) -> &[TextureTableEntry] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.texture_table_offset, packed.texture_table_len) }
    }

    pub fn texture_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.texture_data_offset, packed.texture_data_len) }
    }

    pub fn lightmap_cluster_table(&self) -> &[ClusterLightmapTableEntry] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.lightmap_cluster_table_offset,
                packed.lightmap_cluster_table_len,
            )
        }
    }

    pub fn lightmap_displacement_table(&self) -> &[DisplacementLightmapTableEntry] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.lightmap_displacement_table_offset,
                packed.lightmap_displacement_table_len,
            )
        }
    }

    pub fn lightmap_patch_table(&self) -> &[LightmapPatchTableEntry] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.lightmap_patch_table_offset,
                packed.lightmap_patch_table_len,
            )
        }
    }

    pub fn lightmap_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe { self.cast_slice(packed.lightmap_data_offset, packed.lightmap_data_len) }
    }

    pub fn displacement_position_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.displacement_position_data_offset,
                packed.displacement_position_data_len,
            )
        }
    }

    pub fn displacement_vertex_color_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.displacement_vertex_color_data_offset,
                packed.displacement_vertex_color_data_len,
            )
        }
    }

    pub fn displacement_texture_coordinate_data(&self) -> &[u8] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.displacement_texture_coordinate_data_offset,
                packed.displacement_texture_coordinate_data_len,
            )
        }
    }

    pub fn displacement_table(&self) -> &[DisplacementTableEntry] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.displacement_table_offset,
                packed.displacement_table_len,
            )
        }
    }

    pub fn displacement_byte_code(&self) -> &[u32] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.displacement_byte_code_offset,
                packed.displacement_byte_code_len,
            )
        }
    }

    pub fn displacement_display_lists(&self) -> &[u8] {
        let packed = self.packed();
        unsafe {
            self.cast_slice(
                packed.displacement_display_lists_offset,
                packed.displacement_display_lists_len,
            )
        }
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ClusterGeometryTableEntry {
    pub byte_code_index_ranges: [[u32; 2]; 17],
}

impl ClusterGeometryTableEntry {
    pub fn iter_display_lists<'a>(
        &'a self,
        cluster_geometry_byte_code: &'a [u32],
        pass: usize,
    ) -> impl Iterator<Item = BytecodeOp> + 'a {
        let start = self.byte_code_index_ranges[pass][0] as usize;
        let end = self.byte_code_index_ranges[pass][1] as usize;
        BytecodeReader::new(&cluster_geometry_byte_code[start..end])
    }
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for ClusterGeometryTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        for range in self.byte_code_index_ranges.iter() {
            for &index in range {
                w.write_u32::<BigEndian>(index)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct BspNode {
    pub plane: [f32; 4],
    pub children: [i32; 2],
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for BspNode {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(self.plane[0].to_bits())?;
        w.write_u32::<BigEndian>(self.plane[1].to_bits())?;
        w.write_u32::<BigEndian>(self.plane[2].to_bits())?;
        w.write_u32::<BigEndian>(self.plane[3].to_bits())?;
        w.write_i32::<BigEndian>(self.children[0])?;
        w.write_i32::<BigEndian>(self.children[1])?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct BspLeaf {
    pub cluster: i16,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for BspLeaf {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_i16::<BigEndian>(self.cluster)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct TextureTableEntry {
    pub width: u16,
    pub height: u16,
    pub mip_count: u8,
    pub flags: u8,
    /// One of the GX_TF_* enumerated values.
    pub format: u8,
    pub _padding: u8,
    pub start_offset: u32,
    pub end_offset: u32,
}

impl TextureTableEntry {
    pub const FLAG_CLAMP_S: u8 = 0x01;
    pub const FLAG_CLAMP_T: u8 = 0x02;
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for TextureTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u16::<BigEndian>(self.width)?;
        w.write_u16::<BigEndian>(self.height)?;
        w.write_u8(self.mip_count)?;
        w.write_u8(self.flags)?;
        w.write_u8(self.format)?;
        w.write_u8(self._padding)?;
        w.write_u32::<BigEndian>(self.start_offset)?;
        w.write_u32::<BigEndian>(self.end_offset)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CommonLightmapTableEntry {
    pub width: u16,
    pub height: u16,
    pub patch_table_start_index: u32,
    pub patch_table_end_index: u32,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for CommonLightmapTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u16::<BigEndian>(self.width)?;
        w.write_u16::<BigEndian>(self.height)?;
        w.write_u32::<BigEndian>(self.patch_table_start_index)?;
        w.write_u32::<BigEndian>(self.patch_table_end_index)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ClusterLightmapTableEntry {
    pub common: CommonLightmapTableEntry,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for ClusterLightmapTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        self.common.write_to(w)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct DisplacementLightmapTableEntry {
    pub face_index: u16,
    pub _padding: u16,
    pub common: CommonLightmapTableEntry,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for DisplacementLightmapTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u16::<BigEndian>(self.face_index)?;
        w.write_u16::<BigEndian>(self._padding)?;
        self.common.write_to(w)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct LightmapPatchTableEntry {
    pub sub_block_x: u8,
    pub sub_block_y: u8,
    pub sub_blocks_wide: u8,
    pub sub_blocks_high: u8,
    pub style_count: u8,
    pub _padding1: u8,
    pub _padding2: u16,
    pub data_start_offset: u32,
    pub data_end_offset: u32,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for LightmapPatchTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u8(self.sub_block_x)?;
        w.write_u8(self.sub_block_y)?;
        w.write_u8(self.sub_blocks_wide)?;
        w.write_u8(self.sub_blocks_high)?;
        w.write_u8(self.style_count)?;
        w.write_u8(self._padding1)?;
        w.write_u16::<BigEndian>(self._padding2)?;
        w.write_u32::<BigEndian>(self.data_start_offset)?;
        w.write_u32::<BigEndian>(self.data_end_offset)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct DisplacementTableEntry {
    pub byte_code_start_index: u32,
    pub byte_code_end_index: u32,
}

#[cfg(feature = "std")]
impl<W: Seek + Write> WriteTo<W> for DisplacementTableEntry {
    fn write_to(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(self.byte_code_start_index)?;
        w.write_u32::<BigEndian>(self.byte_code_end_index)?;
        Ok(())
    }
}
