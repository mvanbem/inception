use std::mem::size_of;
use std::num::NonZeroUsize;

use nalgebra_glm::Vec3;
use recursive_iter::*;

use crate::transmute_utils::{extract_at, extract_slice};

#[derive(Clone, Copy)]
pub struct Bsp<'a>(&'a [u8]);

impl<'a> Bsp<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn header(&self) -> &'a Header {
        // SAFETY: All bit patterns are valid for Header.
        unsafe { extract_at(self.0, 0) }
    }

    pub fn vertices(&self) -> &'a [Vec3] {
        // SAFETY: All bit patterns are valid for Vec3.
        unsafe { extract_slice(self.header().lumps[3].data(self.0)) }
    }

    pub fn nodes(&self) -> &'a [Node] {
        // SAFETY: All bit patterns are valid for Node.
        unsafe { extract_slice(self.header().lumps[5].data(self.0)) }
    }

    pub fn tex_infos(&self) -> &'a [TexInfo] {
        // SAFETY: All bit patterns are valid for TexInfo.
        unsafe { extract_slice(self.header().lumps[6].data(self.0)) }
    }

    pub fn faces(&self) -> &'a [Face] {
        // SAFETY: All bit patterns are valid for Face.
        unsafe { extract_slice(self.header().lumps[7].data(self.0)) }
    }

    pub fn lighting(&self) -> Lighting<'a> {
        Lighting {
            data: self.header().lumps[8].data(self.0),
        }
    }

    pub fn leaves(&self) -> &'a [Leaf] {
        // SAFETY: All bit patterns are valid for Leaf.
        unsafe { extract_slice(self.header().lumps[10].data(self.0)) }
    }

    pub fn edges(&self) -> &'a [Edge] {
        // SAFETY: All bit patterns are valid for Edge.
        unsafe { extract_slice(self.header().lumps[12].data(self.0)) }
    }

    pub fn surf_edges(&self) -> &'a [i32] {
        // SAFETY: All bit patterns are valid for i32.
        unsafe { extract_slice(self.header().lumps[13].data(self.0)) }
    }

    pub fn leaf_faces(&self) -> &'a [u16] {
        // SAFETY: All bit patterns are valid for u16.
        unsafe { extract_slice(self.header().lumps[16].data(self.0)) }
    }

    pub fn iter_worldspawn_leaves(&self) -> impl Iterator<Item = &'a Leaf> {
        self.iter_leaves_from_node(&self.nodes()[0])
    }

    pub fn iter_leaves_from_node(&self, node: &'a Node) -> impl Iterator<Item = &'a Leaf> {
        RecursiveIter::new(
            *self,
            LeavesIterFrame {
                node: node,
                child_index: 0,
            },
        )
    }

    pub fn iter_faces_from_leaf(&self, leaf: &'a Leaf) -> impl Iterator<Item = &'a Face> {
        let leaf_face_index = leaf.first_leaf_face as usize;
        LeafFacesIter {
            bsp: *self,
            leaf_face_index,
            end: NonZeroUsize::new(leaf_face_index)
                .map(|start| start.get() + leaf.num_leaf_faces as usize + 1)
                .unwrap_or(0),
        }
    }

    pub fn iter_vertex_indices_from_face(
        &self,
        face: &'a Face,
    ) -> impl Iterator<Item = usize> + 'a {
        let surf_edge_index = face.first_edge as usize;
        FaceVertexIndicesIter {
            bsp: *self,
            surf_edge_index,
            end: surf_edge_index + face.num_edges as usize,
            first_edge_trailing_vertex_index: None,
            prev_edge_leading_vertex_index: None,
        }
    }
}

struct LeavesIterFrame<'a> {
    node: &'a Node,
    child_index: usize,
}

impl<'a> Frame for LeavesIterFrame<'a> {
    type Item = &'a Leaf;
    type Context = Bsp<'a>;

    fn eval(&mut self, bsp: &mut Bsp<'a>) -> EvalResult<Self> {
        let child = self.node.children[self.child_index];
        if child > 0 {
            self.child_index += 1;
            Call(Self {
                node: &bsp.nodes()[child as usize],
                child_index: 0,
            })
            .with_return(self.child_index == 2)
        } else {
            self.child_index += 1;
            Yield(&bsp.leaves()[(-child) as usize]).with_return(self.child_index == 2)
        }
    }
}

struct LeafFacesIter<'a> {
    bsp: Bsp<'a>,
    leaf_face_index: usize,
    end: usize,
}

impl<'a> Iterator for LeafFacesIter<'a> {
    type Item = &'a Face;

    fn next(&mut self) -> Option<&'a Face> {
        if self.leaf_face_index < self.end {
            let face_index = self.bsp.leaf_faces()[self.leaf_face_index] as usize;
            self.leaf_face_index += 1;
            Some(&self.bsp.faces()[face_index])
        } else {
            None
        }
    }
}

struct FaceVertexIndicesIter<'a> {
    bsp: Bsp<'a>,
    surf_edge_index: usize,
    end: usize,
    first_edge_trailing_vertex_index: Option<usize>,
    prev_edge_leading_vertex_index: Option<usize>,
}

impl<'a> Iterator for FaceVertexIndicesIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.surf_edge_index < self.end {
            // Look up the "surfedge", an oriented reference to an edge.
            let surf_edge = self.bsp.surf_edges()[self.surf_edge_index];
            self.surf_edge_index += 1;
            let flipped = if surf_edge < 0 { 1 } else { 0 };
            let edge_index = surf_edge.abs() as usize;

            // Resolve the surfedge to a pair of vertex indices.
            let edge = &self.bsp.edges()[edge_index];
            let trailing_vertex_index = edge.v[0 ^ flipped] as usize;
            let leading_vertex_index = edge.v[1 ^ flipped] as usize;

            // This edge should link with the previous edge, if there was one.
            if let Some(prev_edge_leading_vertex_index) = self.prev_edge_leading_vertex_index {
                assert_eq!(trailing_vertex_index, prev_edge_leading_vertex_index);
            }
            // The final edge should link back to the first edge.
            if self.surf_edge_index == self.end {
                assert_eq!(
                    Some(leading_vertex_index),
                    self.first_edge_trailing_vertex_index,
                );
            }

            // Take notes for validation.
            self.prev_edge_leading_vertex_index = Some(leading_vertex_index);
            if self.first_edge_trailing_vertex_index.is_none() {
                self.first_edge_trailing_vertex_index = Some(trailing_vertex_index);
            }

            Some(trailing_vertex_index)
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    pub ident: i32,
    pub version: i32,
    pub lumps: [Lump; 64],
    pub map_revision: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct Lump {
    pub fileofs: i32,
    pub filelen: i32,
    pub version: i32,
    pub fourcc: [u8; 4],
}

impl Lump {
    pub fn data<'a>(&self, bsp_data: &'a [u8]) -> &'a [u8] {
        &(&bsp_data[self.fileofs as usize..])[..self.filelen as usize]
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Node {
    pub planenum: i32,
    pub children: [i32; 2],
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    pub first_face: u16,
    pub num_faces: u16,
    pub area: i16,
    pub padding: i16,
}

#[repr(C)]
#[derive(Debug)]
pub struct TexInfo {
    pub texture_vecs: [[f32; 4]; 2],
    pub lightmap_vecs: [[f32; 4]; 2],
    pub flags: i32,
    pub tex_data: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct Face {
    pub plane_num: u16,
    pub side: u8,
    pub on_node: u8,
    pub first_edge: i32,
    pub num_edges: i16,
    pub tex_info: i16,
    pub disp_info: i16,
    pub surface_fog_volume_id: i16,
    pub styles: [u8; 4],
    pub light_ofs: i32,
    pub area: f32,
    pub lightmap_texture_mins_in_luxels: [i32; 2],
    pub lightmap_texture_size_in_luxels: [i32; 2],
    pub orig_face: i32,
    pub num_prims: u16,
    pub first_prim_id: u16,
    pub smoothing_groups: u32,
}

pub struct Lighting<'a> {
    data: &'a [u8],
}

impl<'a> Lighting<'a> {
    pub fn at_offset(&self, offset: i32, count: usize) -> &'a [ColorRgbExp32] {
        // SAFETY: All bit patterns are valid for ColorRgbExp32.
        unsafe {
            extract_slice(&(&self.data[offset as usize..])[..count * size_of::<ColorRgbExp32>()])
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Leaf {
    pub contents: i32,
    pub cluster: i16,
    pub area_and_flags: i16,
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    pub first_leaf_face: u16,
    pub num_leaf_faces: u16,
    pub first_leaf_brush: u16,
    pub num_leaf_brushes: u16,
    pub leaf_water_data_id: i16,
    pub ambient_lighting: CompressedLightCube,
    pub padding: i16,
}

#[repr(C)]
#[derive(Debug)]
pub struct Edge {
    pub v: [u16; 2],
}

pub type CompressedLightCube = [ColorRgbExp32; 6];

#[repr(C)]
#[derive(Debug)]
pub struct ColorRgbExp32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub exponent: i8,
}

impl ColorRgbExp32 {
    pub fn to_rgb8(&self) -> [u8; 3] {
        let map = |x| (x as f32 * (self.exponent as f32).exp2()).clamp(0.0, 255.0) as u8;
        [map(self.r), map(self.g), map(self.b)]
    }
}

#[cfg(test)]
mod size_tests {
    use super::{Face, Leaf, Node, TexInfo};

    #[test]
    fn node_size() {
        assert_eq!(std::mem::size_of::<Node>(), 32);
    }

    #[test]
    fn texinfo_size() {
        assert_eq!(std::mem::size_of::<TexInfo>(), 72);
    }

    #[test]
    fn leaf_size() {
        assert_eq!(std::mem::size_of::<Leaf>(), 56);
    }

    #[test]
    fn face_size() {
        assert_eq!(std::mem::size_of::<Face>(), 56);
    }
}
