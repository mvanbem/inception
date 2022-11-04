use std::fmt::{self, Debug, Formatter};
use std::mem::size_of;

use bytemuck::{from_bytes, Pod, Zeroable};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Clone, Copy)]
pub struct Vtx<'a>(&'a [u8]);

impl<'a> Vtx<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn header(self) -> &'a Header {
        from_bytes(&self.0[..size_of::<Header>()])
    }

    pub fn body_parts(self) -> &'a [BodyPart] {
        let header = self.header();
        let bytes = &self.0[header.body_part_offset as usize..]
            [..header.num_body_parts as usize * size_of::<BodyPart>()];
        bytemuck::cast_slice(bytes)
    }

    fn offset_of<T>(self, t: &T) -> usize {
        let ptr = t as *const T as *const u8;
        let bounds = self.0.as_ptr_range();
        assert!(bounds.contains(&ptr));
        ptr as usize - bounds.start as usize
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Header {
    pub version: i32,
    pub vert_cache_size: i32,
    pub max_bones_per_strip: u16,
    pub max_bones_per_tri: u16,
    pub max_bones_per_vert: i32,
    pub check_sum: i32,
    pub num_lods: i32,
    pub material_replacement_list_offset: i32,
    pub num_body_parts: i32,
    pub body_part_offset: i32,
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct BodyPart {
    pub num_models: i32,
    pub model_offset: i32,
}

impl BodyPart {
    pub fn models<'a>(&self, vtx: Vtx<'a>) -> &'a [ModelHeader] {
        let bytes = &vtx.0[vtx.offset_of(self) + self.model_offset as usize..]
            [..self.num_models as usize * size_of::<ModelHeader>()];
        bytemuck::cast_slice(bytes)
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct ModelHeader {
    pub num_lods: i32,
    pub lod_offset: i32,
}

impl ModelHeader {
    pub fn lods<'a>(&self, vtx: Vtx<'a>) -> &'a [ModelLodHeader] {
        let bytes = &vtx.0[vtx.offset_of(self) + self.lod_offset as usize..]
            [..self.num_lods as usize * size_of::<ModelLodHeader>()];
        bytemuck::cast_slice(bytes)
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct ModelLodHeader {
    pub num_meshes: i32,
    pub mesh_offset: i32,
    pub switch_point: f32,
}

impl ModelLodHeader {
    pub fn iter_meshes<'a>(&self, vtx: Vtx<'a>) -> impl Iterator<Item = MeshHeader<'a>> + 'a {
        let offset_of_self = vtx.offset_of(self);
        let mesh_offset = self.mesh_offset as usize;
        (0..(self.num_meshes as usize))
            .into_iter()
            .map(move |index| MeshHeader {
                vtx: vtx.0,
                offset: offset_of_self + mesh_offset + index * MeshHeader::SIZE,
            })
    }
}

#[derive(Clone, Copy)]
pub struct MeshHeader<'a> {
    vtx: &'a [u8],
    offset: usize,
}

impl<'a> MeshHeader<'a> {
    pub const SIZE: usize = 9;

    fn num_strip_groups(self) -> i32 {
        (&self.vtx[self.offset..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn strip_group_header_offset(self) -> i32 {
        (&self.vtx[self.offset + 4..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn flags(self) -> u8 {
        self.vtx[self.offset + 8]
    }

    pub fn iter_strip_groups(self) -> impl Iterator<Item = StripGroupHeader<'a>> + 'a {
        (0..(self.num_strip_groups() as usize))
            .into_iter()
            .map(move |index| StripGroupHeader {
                vtx: self.vtx,
                offset: self.offset
                    + self.strip_group_header_offset() as usize
                    + index * StripGroupHeader::SIZE,
            })
    }
}

impl<'a> Debug for MeshHeader<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("MeshHeader")
            .field("offset", &self.offset)
            .field("num_strip_groups", &self.num_strip_groups())
            .field(
                "strip_group_header_offset",
                &self.strip_group_header_offset(),
            )
            .field("flags", &self.flags())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct StripGroupHeader<'a> {
    vtx: &'a [u8],
    offset: usize,
}

impl<'a> StripGroupHeader<'a> {
    pub const SIZE: usize = 25;

    fn num_verts(self) -> i32 {
        (&self.vtx[self.offset..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn vert_offset(self) -> i32 {
        (&self.vtx[self.offset + 4..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn num_indices(self) -> i32 {
        (&self.vtx[self.offset + 8..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn index_offset(self) -> i32 {
        (&self.vtx[self.offset + 12..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn num_strips(self) -> i32 {
        (&self.vtx[self.offset + 16..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn strip_offset(self) -> i32 {
        (&self.vtx[self.offset + 20..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    fn flags(self) -> u8 {
        self.vtx[self.offset + 24]
    }

    pub fn vert(self, index: usize) -> Vertex<'a> {
        assert!(index < self.num_verts() as usize);
        Vertex {
            vtx: self.vtx,
            offset: self.offset + self.vert_offset() as usize + index * Vertex::SIZE,
        }
    }

    pub fn iter_verts(self) -> impl Iterator<Item = Vertex<'a>> + 'a {
        (0..(self.num_verts() as usize))
            .into_iter()
            .map(move |index| self.vert(index))
    }

    pub fn index(self, index: usize) -> u16 {
        assert!(index < self.num_indices() as usize);
        (&self.vtx[self.offset + self.index_offset() as usize + index * size_of::<u16>()..][..2])
            .read_u16::<LittleEndian>()
            .unwrap()
    }

    pub fn iter_indices(self) -> impl Iterator<Item = u16> + 'a {
        (0..(self.num_indices() as usize))
            .into_iter()
            .map(move |index| self.index(index))
    }

    pub fn strip(self, index: usize) -> StripHeader<'a> {
        assert!(index < self.num_strips() as usize);
        StripHeader {
            vtx: self.vtx,
            offset: self.offset + self.strip_offset() as usize + index * StripHeader::SIZE,
        }
    }

    pub fn iter_strips(self) -> impl Iterator<Item = StripHeader<'a>> + 'a {
        (0..(self.num_strips() as usize))
            .into_iter()
            .map(move |index| self.strip(index))
    }
}

impl<'a> Debug for StripGroupHeader<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("StripGroupHeader")
            .field("offset", &self.offset)
            .field("num_verts", &self.num_verts())
            .field("vert_offset", &self.vert_offset())
            .field("num_indices", &self.num_indices())
            .field("index_offset", &self.index_offset())
            .field("num_strips", &self.num_strips())
            .field("strip_offset", &self.strip_offset())
            .field("flags", &self.flags())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct Vertex<'a> {
    vtx: &'a [u8],
    offset: usize,
}

impl<'a> Vertex<'a> {
    pub const SIZE: usize = 9;

    pub fn bone_weight_index(self) -> [u8; 3] {
        (&self.vtx[self.offset..][..3]).try_into().unwrap()
    }

    pub fn num_bones(self) -> u8 {
        self.vtx[self.offset + 3]
    }

    pub fn orig_mesh_vert_id(self) -> u16 {
        (&self.vtx[self.offset + 4..][..2])
            .read_u16::<LittleEndian>()
            .unwrap()
    }

    pub fn bone_id(self) -> [u8; 3] {
        (&self.vtx[self.offset + 6..][..3]).try_into().unwrap()
    }
}

impl<'a> Debug for Vertex<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Vertex")
            .field("offset", &self.offset)
            .field("bone_weight_index", &self.bone_weight_index())
            .field("num_bones", &self.num_bones())
            .field("orig_mesh_vert_id", &self.orig_mesh_vert_id())
            .field("bone_id", &self.bone_id())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct StripHeader<'a> {
    vtx: &'a [u8],
    offset: usize,
}

impl<'a> StripHeader<'a> {
    const SIZE: usize = 27;

    pub fn num_indices(self) -> i32 {
        (&self.vtx[self.offset..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn index_offset(self) -> i32 {
        (&self.vtx[self.offset + 4..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn num_verts(self) -> i32 {
        (&self.vtx[self.offset + 8..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn vert_offset(self) -> i32 {
        (&self.vtx[self.offset + 12..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn num_bones(self) -> u16 {
        (&self.vtx[self.offset + 16..][..2])
            .read_u16::<LittleEndian>()
            .unwrap()
    }

    pub fn flags(self) -> StripHeaderFlags {
        StripHeaderFlags(self.vtx[self.offset + 18])
    }

    pub fn num_bone_state_changes(self) -> i32 {
        (&self.vtx[self.offset + 19..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn bone_state_change_offset(self) -> i32 {
        (&self.vtx[self.offset + 23..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn bone_state_change(self, index: usize) -> BoneStateChangeHeader<'a> {
        assert!(index < self.num_bone_state_changes() as usize);
        BoneStateChangeHeader {
            vtx: self.vtx,
            offset: self.offset
                + self.bone_state_change_offset() as usize
                + index * BoneStateChangeHeader::SIZE,
        }
    }

    pub fn iter_bone_state_changes(self) -> impl Iterator<Item = BoneStateChangeHeader<'a>> + 'a {
        (0..(self.num_bone_state_changes() as usize))
            .into_iter()
            .map(move |index| self.bone_state_change(index))
    }
}

impl<'a> Debug for StripHeader<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("StripHeader")
            .field("offset", &self.offset)
            .field("num_indices", &self.num_indices())
            .field("index_offset", &self.index_offset())
            .field("num_verts", &self.num_verts())
            .field("vert_offset", &self.vert_offset())
            .field("num_bones", &self.num_bones())
            .field("flags", &self.flags())
            .field("num_bone_state_changes", &self.num_bone_state_changes())
            .field("bone_state_change_offset", &self.bone_state_change_offset())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct StripHeaderFlags(pub u8);

impl StripHeaderFlags {
    pub fn is_trilist(self) -> bool {
        (self.0 & 1) != 0
    }

    pub fn is_tristrip(self) -> bool {
        (self.0 & 2) != 0
    }
}

impl Debug for StripHeaderFlags {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let flags: &[(&str, fn(Self) -> bool)] = &[
            ("IS_TRILIST", |x| x.is_trilist()),
            ("IS_TRISTRIP", |x| x.is_tristrip()),
            ("UNKNOWN_BITS", |x| (x.0 & !3) != 0),
        ];
        let mut any = false;
        for &(name, predicate) in flags {
            if predicate(*self) {
                if any {
                    write!(f, " | ")?;
                }
                any = true;
                write!(f, "{}", name)?;
            }
        }
        if !any {
            write!(f, "0")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct BoneStateChangeHeader<'a> {
    vtx: &'a [u8],
    offset: usize,
}

impl<'a> BoneStateChangeHeader<'a> {
    const SIZE: usize = 8;

    pub fn hardware_id(self) -> i32 {
        (&self.vtx[self.offset..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }

    pub fn new_bone_id(self) -> i32 {
        (&self.vtx[self.offset + 4..][..4])
            .read_i32::<LittleEndian>()
            .unwrap()
    }
}

impl<'a> Debug for BoneStateChangeHeader<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("BoneStateChangeHeader")
            .field("offset", &self.offset)
            .field("hardware_id", &self.hardware_id())
            .field("new_bone_id", &self.new_bone_id())
            .finish()
    }
}
