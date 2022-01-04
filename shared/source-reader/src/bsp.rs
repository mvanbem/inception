use std::collections::HashMap;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem::size_of;
use std::num::NonZeroUsize;
use std::str;

use byteorder::{LittleEndian, ReadBytesExt};
use nalgebra_glm::{vec3, Vec3};
use recursive_iter::*;
use zip::ZipArchive;

use fully_occupied::{extract, extract_slice, extract_slice_unchecked, FullyOccupied};

use crate::properties;

#[derive(Clone, Copy)]
pub struct Bsp<'a>(&'a [u8]);

impl<'a> Bsp<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn header(self) -> &'a Header {
        extract(self.0)
    }

    pub fn lump_data(&self, index: usize) -> &'a [u8] {
        self.header().lumps[index].data(self.0)
    }

    pub fn entities(self) -> Vec<HashMap<String, String>> {
        let bytes = self.header().lumps[0].data(self.0);
        assert_eq!(bytes[bytes.len() - 1], 0);
        let bytes = &bytes[..bytes.len() - 1];
        properties::flat_objects(str::from_utf8(bytes).unwrap()).unwrap()
    }

    pub fn planes(self) -> &'a [Plane] {
        extract_slice(self.header().lumps[1].data(self.0))
    }

    pub fn tex_datas(self) -> &'a [TexData] {
        extract_slice(self.header().lumps[2].data(self.0))
    }

    pub fn vertices(self) -> &'a [Vec3] {
        // SAFETY: All bit patterns are valid for Vec3.
        unsafe { extract_slice_unchecked(self.header().lumps[3].data(self.0)) }
    }

    pub fn visibility(self) -> Visibility<'a> {
        Visibility {
            data: self.header().lumps[4].data(self.0),
        }
    }

    pub fn nodes(self) -> &'a [Node] {
        extract_slice(self.header().lumps[5].data(self.0))
    }

    pub fn tex_infos(self) -> &'a [TexInfo] {
        extract_slice(self.header().lumps[6].data(self.0))
    }

    pub fn faces(self) -> &'a [Face] {
        let ldr_lighting_lump = &self.header().lumps[8];
        extract_slice(if ldr_lighting_lump.filelen == 0 {
            // No LDR lighting, so fall back to HDR lighting and faces.
            self.header().lumps[58].data(self.0)
        } else {
            // Otherwise use the LDR faces.
            self.header().lumps[7].data(self.0)
        })
    }

    pub fn lighting(self) -> Lighting<'a> {
        let ldr_lighting_lump = &self.header().lumps[8];
        let hdr_lighting_lump = &self.header().lumps[53];

        Lighting {
            data: if ldr_lighting_lump.filelen == 0 {
                hdr_lighting_lump.data(self.0)
            } else {
                ldr_lighting_lump.data(self.0)
            },
        }
    }

    pub fn leaves(self) -> LeafSlice<'a> {
        let leaf_data = self.header().lumps[10].data(self.0);
        match self.header().version {
            20 => LeafSlice::Short(extract_slice(leaf_data)),
            19 => LeafSlice::Long(extract_slice(leaf_data)),
            version => panic!(
                "Unknown BSP version {} with leaf lump size: {}",
                version,
                leaf_data.len(),
            ),
        }
    }

    pub fn edges(self) -> &'a [Edge] {
        extract_slice(self.header().lumps[12].data(self.0))
    }

    pub fn surf_edges(self) -> &'a [i32] {
        extract_slice(self.header().lumps[13].data(self.0))
    }

    pub fn leaf_faces(self) -> &'a [u16] {
        extract_slice(self.header().lumps[16].data(self.0))
    }

    pub fn disp_infos(self) -> &'a [DispInfo] {
        extract_slice(self.header().lumps[26].data(self.0))
    }

    pub fn disp_verts(self) -> &'a [DispVert] {
        extract_slice(self.header().lumps[33].data(self.0))
    }

    pub fn pak_file(self) -> ZipArchive<Cursor<&'a [u8]>> {
        ZipArchive::new(Cursor::new(self.header().lumps[40].data(self.0))).unwrap()
    }

    pub fn tex_data_strings(self) -> TexDataStrings<'a> {
        let table: &[i32] = extract_slice(self.header().lumps[44].data(self.0));
        let data = self.header().lumps[43].data(self.0);
        TexDataStrings { table, data }
    }

    pub fn disp_tris(self) -> &'a [DispTri] {
        extract_slice(self.header().lumps[48].data(self.0))
    }

    pub fn iter_worldspawn_leaves(self) -> impl Iterator<Item = &'a dyn Leaf> {
        self.enumerate_leaves_from_node(&self.nodes()[0])
    }

    pub fn enumerate_leaves_from_node(self, node: &'a Node) -> impl Iterator<Item = &'a dyn Leaf> {
        RecursiveIter::new(
            self,
            LeavesIterFrame {
                node: node,
                child_index: 0,
            },
        )
    }

    pub fn iter_faces_from_leaf(self, leaf: &'a dyn Leaf) -> impl Iterator<Item = &'a Face> {
        let leaf_face_index = leaf.first_leaf_face() as usize;
        LeafFacesIter {
            bsp: self,
            leaf_face_index,
            end: NonZeroUsize::new(leaf_face_index)
                .map(|start| start.get() + leaf.num_leaf_faces() as usize + 1)
                .unwrap_or(0),
        }
    }

    pub fn iter_vertex_indices_from_face(self, face: &'a Face) -> impl Iterator<Item = usize> + 'a {
        let surf_edge_index = face.first_edge as usize;
        FaceVertexIndicesIter {
            bsp: self,
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
    type Item = &'a dyn Leaf;
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
            Yield(bsp.leaves().get((-child) as usize)).with_return(self.child_index == 2)
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

unsafe impl FullyOccupied for Header {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
pub struct Plane {
    pub normal: [f32; 3],
    pub dist: f32,
    pub type_: i32,
}

unsafe impl FullyOccupied for Plane {}

#[repr(C)]
pub struct TexData {
    pub reflectivity: [f32; 3],
    pub name_string_table_id: i32,
    pub width: i32,
    pub height: i32,
    pub view_width: i32,
    pub view_height: i32,
}

unsafe impl FullyOccupied for TexData {}

#[derive(Clone, Copy)]
pub struct Visibility<'a> {
    data: &'a [u8],
}

impl<'a> Visibility<'a> {
    pub fn num_clusters(self) -> usize {
        (&self.data[..]).read_i32::<LittleEndian>().unwrap() as usize
    }

    pub fn get_cluster(self, index: ClusterIndex) -> VisibilityBitmap<'a> {
        let num_clusters = self.num_clusters();
        assert!(index.0 < num_clusters);
        let pvs_byte_ofs = (&self.data[8 * index.0 + 4..])
            .read_i32::<LittleEndian>()
            .unwrap() as usize;
        VisibilityBitmap {
            data: &self.data[pvs_byte_ofs..],
            num_clusters,
        }
    }

    pub fn iter_clusters(self) -> impl Iterator<Item = VisibilityBitmap<'a>> {
        (0..self.num_clusters())
            .into_iter()
            .map(move |index| self.get_cluster(ClusterIndex(index)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClusterIndex(pub usize);

#[derive(Clone, Copy)]
pub struct VisibilityBitmap<'a> {
    data: &'a [u8],
    num_clusters: usize,
}

impl<'a> VisibilityBitmap<'a> {
    pub fn iter_visible_clusters(self) -> impl Iterator<Item = ClusterIndex> + 'a {
        VisibilityBitmapIter {
            data: self.data,
            cluster_index: 0,
            num_clusters: self.num_clusters,
            current_byte: 0,
            current_bit: 0,
        }
    }

    pub fn find_data(self) -> &'a [u8] {
        // Length is unknown. Walk the data to find out.
        let mut offset = 0;
        let mut cluster_index = 0;
        loop {
            // Exit on reaching the end of the bitstream.
            if cluster_index >= self.num_clusters {
                return &self.data[..offset];
            }

            // Read a byte, which is either a skip instruction or another eight bits to scan.
            let b = self.data[offset];
            offset += 1;
            if b == 0 {
                let run_len = self.data[offset] as usize;
                offset += 1;
                cluster_index += 8 * run_len;
            } else {
                cluster_index += 8;
            }
        }
    }
}

pub struct VisibilityBitmapIter<'a> {
    data: &'a [u8],
    cluster_index: usize,
    num_clusters: usize,
    current_byte: u8,
    current_bit: u8,
}

impl<'a> Iterator for VisibilityBitmapIter<'a> {
    type Item = ClusterIndex;

    fn next(&mut self) -> Option<ClusterIndex> {
        loop {
            // Exit on reaching the end of the bitstream.
            if self.cluster_index >= self.num_clusters {
                return None;
            }

            // Scan bits until exhausted, yielding any that are set.
            while self.current_bit != 0 {
                let visible = (self.current_byte & self.current_bit) != 0;
                self.current_bit <<= 1;
                let cluster_index = self.cluster_index;
                self.cluster_index += 1;
                if visible {
                    return Some(ClusterIndex(cluster_index));
                }
            }

            // Read another byte, which is either a skip instruction or another eight bits to scan.
            match self.data.read_u8().unwrap() {
                0 => {
                    let run_len = self.data.read_u8().unwrap() as usize;
                    self.cluster_index += 8 * run_len;
                }
                x => {
                    self.current_byte = x;
                    self.current_bit = 1;
                }
            }
        }
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

unsafe impl FullyOccupied for Node {}

#[repr(C)]
#[derive(Debug)]
pub struct TexInfo {
    pub texture_vecs: [[f32; 4]; 2],
    pub lightmap_vecs: [[f32; 4]; 2],
    pub flags: i32,
    pub tex_data: i32,
}

unsafe impl FullyOccupied for TexInfo {}

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

unsafe impl FullyOccupied for Face {}

pub struct Lighting<'a> {
    data: &'a [u8],
}

impl<'a> Lighting<'a> {
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn at_offset(&self, offset: usize, count: usize) -> &'a [ColorRgbExp32] {
        extract_slice(&(&self.data[offset..])[..count * size_of::<ColorRgbExp32>()])
    }
}

pub trait Leaf {
    fn contents(&self) -> i32;
    fn cluster(&self) -> i16;
    fn area_and_flags(&self) -> i16;
    fn mins(&self) -> [i16; 3];
    fn maxs(&self) -> [i16; 3];
    fn first_leaf_face(&self) -> u16;
    fn num_leaf_faces(&self) -> u16;
    fn first_leaf_brush(&self) -> u16;
    fn num_leaf_brushes(&self) -> u16;
    fn leaf_water_data_id(&self) -> i16;
}

#[derive(Clone, Copy)]
pub enum LeafSlice<'a> {
    Short(&'a [ShortLeaf]),
    Long(&'a [LongLeaf]),
}

impl<'a> LeafSlice<'a> {
    fn get(&self, index: usize) -> &'a dyn Leaf {
        match self {
            Self::Short(slice) => &slice[index],
            Self::Long(slice) => &slice[index],
        }
    }

    fn iter(&self) -> LeafSliceIter<'a> {
        LeafSliceIter(*self)
    }
}

impl<'a> IntoIterator for LeafSlice<'a> {
    type Item = &'a dyn Leaf;

    type IntoIter = LeafSliceIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct LeafSliceIter<'a>(LeafSlice<'a>);

impl<'a> Iterator for LeafSliceIter<'a> {
    type Item = &'a dyn Leaf;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            LeafSlice::Short(ref mut slice) => match slice.get(0) {
                Some(item) => {
                    *slice = &slice[1..];
                    Some(item)
                }
                None => None,
            },
            LeafSlice::Long(ref mut slice) => match slice.get(0) {
                Some(item) => {
                    *slice = &slice[1..];
                    Some(item)
                }
                None => None,
            },
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShortLeaf {
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
}

unsafe impl FullyOccupied for ShortLeaf {}

impl Leaf for ShortLeaf {
    fn contents(&self) -> i32 {
        self.contents
    }

    fn cluster(&self) -> i16 {
        self.cluster
    }

    fn area_and_flags(&self) -> i16 {
        self.area_and_flags
    }

    fn mins(&self) -> [i16; 3] {
        self.mins
    }

    fn maxs(&self) -> [i16; 3] {
        self.maxs
    }

    fn first_leaf_face(&self) -> u16 {
        self.first_leaf_face
    }

    fn num_leaf_faces(&self) -> u16 {
        self.num_leaf_faces
    }

    fn first_leaf_brush(&self) -> u16 {
        self.first_leaf_brush
    }

    fn num_leaf_brushes(&self) -> u16 {
        self.num_leaf_brushes
    }

    fn leaf_water_data_id(&self) -> i16 {
        self.leaf_water_data_id
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct LongLeaf {
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

unsafe impl FullyOccupied for LongLeaf {}

impl Leaf for LongLeaf {
    fn contents(&self) -> i32 {
        self.contents
    }

    fn cluster(&self) -> i16 {
        self.cluster
    }

    fn area_and_flags(&self) -> i16 {
        self.area_and_flags
    }

    fn mins(&self) -> [i16; 3] {
        self.mins
    }

    fn maxs(&self) -> [i16; 3] {
        self.maxs
    }

    fn first_leaf_face(&self) -> u16 {
        self.first_leaf_face
    }

    fn num_leaf_faces(&self) -> u16 {
        self.num_leaf_faces
    }

    fn first_leaf_brush(&self) -> u16 {
        self.first_leaf_brush
    }

    fn num_leaf_brushes(&self) -> u16 {
        self.num_leaf_brushes
    }

    fn leaf_water_data_id(&self) -> i16 {
        self.leaf_water_data_id
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Edge {
    pub v: [u16; 2],
}

unsafe impl FullyOccupied for Edge {}

#[repr(C)]
#[derive(Debug)]
pub struct DispInfo {
    pub start_position: [f32; 3],
    pub disp_vert_start: i32,
    pub disp_tri_start: i32,
    pub power: i32,
    pub min_tess: i32,
    pub smoothing_angle: f32,
    pub contents: i32,
    pub map_face: u16,
    pub lightmap_alpha_start: i32,
    pub lightmap_sample_position_start: i32,
    pub edge_neighbors: [DispNeighbor; 4],
    pub corner_neighbors: [DispCornerNeighbors; 4],
    pub allowed_verts: [i32; 10],
}

impl DispInfo {
    pub fn start_position_vec(&self) -> Vec3 {
        vec3(
            self.start_position[0],
            self.start_position[1],
            self.start_position[2],
        )
    }
}

unsafe impl FullyOccupied for DispInfo {}

#[repr(C)]
#[derive(Debug)]
pub struct DispNeighbor {
    pub sub_neighbors: [DispSubNeighbor; 2],
}

unsafe impl FullyOccupied for DispNeighbor {}

#[repr(C)]
#[derive(Debug)]
pub struct DispSubNeighbor {
    pub i_neighbor: u16,
    pub neighbor_orientation: NeighborOrientation,
    pub span: NeighborSpan,
    pub neighbor_span: NeighborSpan,
}

unsafe impl FullyOccupied for DispSubNeighbor {}

#[repr(transparent)]
#[derive(Debug)]
pub struct NeighborOrientation(u8);

impl NeighborOrientation {
    pub const ORIENTATION_CCW_0: Self = Self(0);
    pub const ORIENTATION_CCW_90: Self = Self(1);
    pub const ORIENTATION_CCW_180: Self = Self(2);
    pub const ORIENTATION_CCW_270: Self = Self(3);
}

#[repr(transparent)]
#[derive(Debug)]
pub struct NeighborSpan(u8);

impl NeighborSpan {
    pub const CORNER_TO_CORNER: Self = Self(0);
    pub const CORNER_TO_MIDPOINT: Self = Self(1);
    pub const MIDPOINT_TO_CORNER: Self = Self(2);
}

#[repr(C)]
#[derive(Debug)]
pub struct DispCornerNeighbors {
    pub neighbors: [u16; 4],
    pub neighbor_count: u8,
}

unsafe impl FullyOccupied for DispCornerNeighbors {}

#[repr(C)]
#[derive(Debug)]
pub struct DispVert {
    pub vec: [f32; 3],
    pub dist: f32,
    pub alpha: f32,
}

unsafe impl FullyOccupied for DispVert {}

#[repr(C)]
#[derive(Debug)]
pub struct DispTri {
    pub tags: u16,
}

unsafe impl FullyOccupied for DispTri {}

#[derive(Clone, Copy)]
pub struct TexDataStrings<'a> {
    table: &'a [i32],
    data: &'a [u8],
}

impl<'a> TexDataStrings<'a> {
    pub fn get(self, index: usize) -> &'a str {
        let start = usize::try_from(self.table[index]).unwrap();
        let end = start
            + self.data[start..]
                .iter()
                .enumerate()
                .find(|&(_index, &b)| b == 0)
                .unwrap()
                .0
            + 1;
        CStr::from_bytes_with_nul(&self.data[start..end])
            .unwrap()
            .to_str()
            .unwrap()
    }
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

unsafe impl FullyOccupied for ColorRgbExp32 {}

impl ColorRgbExp32 {
    const SCALE: f32 = 0.5;

    fn linear_to_srgb(l: f32) -> f32 {
        if l <= 0.0031308 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        }
    }

    pub fn to_rgb8(&self) -> [u8; 3] {
        let map = |x| {
            ((x as f32 * (self.exponent as f32).exp2() * Self::SCALE).clamp(0.0, 255.0) + 0.5) as u8
        };
        [map(self.r), map(self.g), map(self.b)]
    }

    pub fn to_scaled_linear(&self) -> [u8; 3] {
        // NOTE: This multiplier was observed in the PC fragment shaders. Cut it in half so there's
        // room to clamp on the upper end after multiplying by the base texture.
        const MULTIPLIER: f32 = 4.59479 * 0.5;

        let map = |x| {
            ((x as f32 * (self.exponent as f32).exp2() * Self::SCALE * MULTIPLIER)
                .clamp(0.0, 255.0)
                + 0.5) as u8
        };
        [map(self.r), map(self.g), map(self.b)]
    }

    pub fn to_srgb8(&self) -> [u8; 3] {
        let map = |x| {
            let linear =
                (x as f32 * (self.exponent as f32).exp2() / 255.0 * Self::SCALE).clamp(0.0, 1.0);
            let srgb = Self::linear_to_srgb(linear);
            (srgb * 255.0 + 0.5) as u8
        };
        [map(self.r), map(self.g), map(self.b)]
    }
}

#[cfg(test)]
mod size_tests {
    use std::mem::size_of;

    use crate::bsp::{DispTri, DispVert};

    use super::{
        DispCornerNeighbors, DispInfo, DispNeighbor, DispSubNeighbor, Face, LongLeaf, Node,
        ShortLeaf, TexInfo,
    };

    #[test]
    fn node_size() {
        assert_eq!(size_of::<Node>(), 32);
    }

    #[test]
    fn texinfo_size() {
        assert_eq!(size_of::<TexInfo>(), 72);
    }

    #[test]
    fn short_leaf_size() {
        assert_eq!(size_of::<ShortLeaf>(), 32);
    }

    #[test]
    fn long_leaf_size() {
        assert_eq!(size_of::<LongLeaf>(), 56);
    }

    #[test]
    fn face_size() {
        assert_eq!(size_of::<Face>(), 56);
    }

    #[test]
    fn disp_info_size() {
        assert_eq!(size_of::<DispInfo>(), 176);
        assert_eq!(size_of::<DispNeighbor>(), 12);
        assert_eq!(size_of::<DispSubNeighbor>(), 6);
        assert_eq!(size_of::<DispCornerNeighbors>(), 10);
    }

    #[test]
    fn disp_vert_size() {
        assert_eq!(size_of::<DispVert>(), 20);
    }

    #[test]
    fn disp_tri_size() {
        assert_eq!(size_of::<DispTri>(), 2);
    }
}
