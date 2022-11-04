use std::mem::size_of;

use bytemuck::{cast_slice, from_bytes, Pod, Zeroable};

#[derive(Clone, Copy)]
pub struct Mdl<'a>(&'a [u8]);

impl<'a> Mdl<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn header(self) -> &'a Header {
        from_bytes(&self.0[..size_of::<Header>()])
    }

    pub fn bones(self) -> &'a [Bone] {
        let header = self.header();
        let bytes =
            &self.0[header.boneindex as usize..][..header.numbones as usize * size_of::<Bone>()];
        cast_slice(bytes)
    }

    pub fn textures(self) -> &'a [Texture] {
        let header = self.header();
        let bytes = &self.0[header.textureindex as usize..]
            [..header.numtextures as usize * size_of::<Texture>()];
        cast_slice(bytes)
    }

    pub fn body_parts(self) -> &'a [BodyPart] {
        let header = self.header();
        let bytes = &self.0[header.bodypartindex as usize..]
            [..header.numbodyparts as usize * size_of::<BodyPart>()];
        cast_slice(bytes)
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
    pub id: i32,
    pub version: i32,
    pub checksum: i32,
    pub name: [u8; 64],
    pub length: i32,
    pub eyeposition: [f32; 3],
    pub illumposition: [f32; 3],
    pub hull_min: [f32; 3],
    pub hull_max: [f32; 3],
    pub view_bbmin: [f32; 3],
    pub view_bbmax: [f32; 3],
    pub flags: i32,
    pub numbones: i32,
    pub boneindex: i32,
    pub numbonecontrollers: i32,
    pub bonecontrollerindex: i32,
    pub numhitboxsets: i32,
    pub hitboxsetindex: i32,
    pub numlocalanim: i32,
    pub localanimindex: i32,
    pub numlocalseq: i32,
    pub localseqindex: i32,
    pub activitylistversion: i32,
    pub eventsindexed: i32,
    pub numtextures: i32,
    pub textureindex: i32,
    pub numcdtextures: i32,
    pub cdtextureindex: i32,
    pub numskinref: i32,
    pub numskinfamilies: i32,
    pub skinindex: i32,
    pub numbodyparts: i32,
    pub bodypartindex: i32,
    pub numlocalattachments: i32,
    pub localattachmentindex: i32,
    pub numlocalnodes: i32,
    pub localnodeindex: i32,
    pub localnodenameindex: i32,
    pub numflexdesc: i32,
    pub flexdescindex: i32,
    pub numflexcontrollers: i32,
    pub flexcontrollerindex: i32,
    pub numflexrules: i32,
    pub flexruleindex: i32,
    pub numikchains: i32,
    pub ikchainindex: i32,
    pub nummouths: i32,
    pub mouthindex: i32,
    pub numlocalposeparameters: i32,
    pub localposeparamindex: i32,
    pub surfacepropindex: i32,
    pub keyvalueindex: i32,
    pub keyvaluesize: i32,
    pub numlocalikautoplaylocks: i32,
    pub localikautoplaylockindex: i32,
    pub mass: f32,
    pub contents: i32,
    pub numincludemodels: i32,
    pub includemodelindex: i32,
    pub virtual_model: i32,
    pub szanimblocknameindex: i32,
    pub numanimblocks: i32,
    pub animblockindex: i32,
    pub animblock_model: i32,
    pub bonetablebynameindex: i32,
    pub p_vertex_base: i32,
    pub p_index_base: i32,
    pub constdirectionallightdot: u8,
    pub root_lod: u8,
    pub num_allowed_root_lods: u8,
    pub unused: [u8; 1],
    pub unused4: i32,
    pub numflexcontrollerui: i32,
    pub flexcontrolleruiindex: i32,
    pub fl_vert_anim_fixed_point_scale: f32,
    pub unused3: [i32; 1],
    pub studiohdr2index: i32,
    pub unused2: [i32; 1],
}

impl Header {
    pub fn name(&self) -> &str {
        if self.studiohdr2index != 0 {
            unimplemented!("retrieve name from studiohdr2");
        }

        let null_index = self.name.iter().copied().position(|b| b == 0).unwrap();
        std::str::from_utf8(&self.name[..null_index]).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Bone {
    pub sznameindex: i32,
    pub parent: i32,
    pub bonecontroller: [i32; 6],
    pub pos: [f32; 3],
    pub quat: [f32; 4],
    pub rot: [f32; 3],
    pub posscale: [f32; 3],
    pub rotscale: [f32; 3],
    pub pose_to_bone: [f32; 12],
    pub q_alignment: [f32; 4],
    pub flags: i32,
    pub proctype: i32,
    pub procindex: i32,
    pub physicsbone: i32,
    pub surfacepropidx: i32,
    pub contents: i32,
    pub unused: [i32; 8],
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Texture {
    pub sznameindex: i32,
    pub flags: i32,
    pub used: i32,
    pub _unused1: i32,
    pub _material: i32,
    pub _clientmaterial: i32,
    pub _unused: [i32; 10],
}

impl Texture {
    pub fn name<'a>(&self, mdl: Mdl<'a>) -> &'a str {
        let bytes = &mdl.0[mdl.offset_of(self) + self.sznameindex as usize..];
        let null_index = bytes.iter().copied().position(|b| b == 0).unwrap();
        std::str::from_utf8(&bytes[..null_index]).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct BodyPart {
    pub sznameindex: i32,
    pub nummodels: i32,
    pub base: i32,
    pub modelindex: i32,
}

impl BodyPart {
    pub fn models<'a>(&self, mdl: Mdl<'a>) -> &'a [Model] {
        let bytes = &mdl.0[mdl.offset_of(self) + self.modelindex as usize..]
            [..self.nummodels as usize * size_of::<Model>()];
        cast_slice(bytes)
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Model {
    pub name: [u8; 64],
    pub type_: i32,
    pub boundingradius: f32,
    pub nummeshes: i32,
    pub meshindex: i32,
    pub numvertices: i32,
    pub vertexindex: i32,
    pub tangentsindex: i32,
    pub numattachments: i32,
    pub attachmentindex: i32,
    pub numeyeballs: i32,
    pub eyeballindex: i32,
    pub vertexdata: ModelVertexData,
    pub unused: [i32; 8],
}

impl Model {
    pub fn meshes<'a>(&self, mdl: Mdl<'a>) -> &'a [Mesh] {
        let bytes = &mdl.0[mdl.offset_of(self) + self.meshindex as usize..]
            [..self.nummeshes as usize * size_of::<Mesh>()];
        cast_slice(bytes)
    }
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct ModelVertexData {
    pub p_vertex_data: u32,
    pub p_tangent_data: u32,
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Mesh {
    pub material: i32,
    pub modelindex: i32,
    pub numvertices: i32,
    pub vertexoffset: i32,
    pub numflexes: i32,
    pub flexindex: i32,
    pub materialtype: i32,
    pub materialparam: i32,
    pub meshid: i32,
    pub center: [f32; 3],
    pub vertexdata: MeshVertexData,
    pub unused: [i32; 8],
}

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MeshVertexData {
    pub modelvertexdata: u32,
    pub num_lod_vertexes: [i32; 8],
}
