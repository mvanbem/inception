use std::mem::size_of;

use bytemuck::{cast_slice, from_bytes, Pod, Zeroable};

#[derive(Clone, Copy)]
pub struct Vvd<'a>(&'a [u8]);

impl<'a> Vvd<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn header(self) -> &'a Header {
        from_bytes(&self.0[..size_of::<Header>()])
    }

    pub fn fixups(self) -> &'a [Fixup] {
        let header = self.header();
        let bytes = &self.0[header.fixup_table_start as usize..]
            [..header.num_fixups as usize * size_of::<Fixup>()];
        cast_slice(bytes)
    }

    pub fn vertex(self, index: usize) -> &'a Vertex {
        let header = self.header();
        let bytes = &self.0[header.vertex_data_start as usize + index * size_of::<Vertex>()..]
            [..size_of::<Vertex>()];
        from_bytes(bytes)
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Header {
    pub id: i32,
    pub version: i32,
    pub checksum: i32,
    pub num_lods: i32,
    pub num_lod_vertexes: [i32; 8],
    pub num_fixups: i32,
    pub fixup_table_start: i32,
    pub vertex_data_start: i32,
    pub tangent_data_start: i32,
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Fixup {
    pub lod: i32,
    pub source_vertex_id: i32,
    pub num_vertexes: i32,
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    pub bone_weight: [f32; 3],
    pub bone: [u8; 3],
    pub num_bones: u8,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Tangent {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}
