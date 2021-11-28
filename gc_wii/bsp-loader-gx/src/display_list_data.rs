use fully_occupied::{extract_slice, FullyOccupied};

pub struct TexturePlaneDisplayList<'a> {
    pub texture_index: usize,
    pub plane_index: u16,
    pub reflect_front_paraboloid: &'a [[f32; 4]; 3],
    pub reflect_back_paraboloid: &'a [[f32; 4]; 3],
    pub reflect_paraboloid_z: &'a [[f32; 4]; 3],
    pub display_list: &'static [u8],
}

pub fn iter_display_lists_for_cluster(
    cluster: u16,
) -> impl Iterator<Item = TexturePlaneDisplayList<'static>> {
    display_lists_by_cluster_texture_plane()[cluster as usize].iter_display_lists()
}

static DISPLAY_LISTS_BY_CLUSTER_TEXTURE_PLANE_DATA: &[u8] = include_bytes_align!(
    4,
    "../../../build/display_lists_by_cluster_texture_plane.dat"
);
fn display_lists_by_cluster_texture_plane() -> &'static [DisplayListsByClusterTexturePlaneEntry] {
    extract_slice(DISPLAY_LISTS_BY_CLUSTER_TEXTURE_PLANE_DATA)
}

static DISPLAY_LISTS_BY_TEXTURE_PLANE_DATA: &[u8] =
    include_bytes_align!(4, "../../../build/display_lists_by_texture_plane.dat");
fn display_lists_by_texture_plane() -> &'static [DisplayListsByTexturePlaneEntry] {
    extract_slice(DISPLAY_LISTS_BY_TEXTURE_PLANE_DATA)
}

static DISPLAY_LISTS_BY_PLANE_DATA: &[u8] =
    include_bytes_align!(4, "../../../build/display_lists_by_plane.dat");
fn display_lists_by_plane() -> &'static [DisplayListsByPlaneEntry] {
    extract_slice(DISPLAY_LISTS_BY_PLANE_DATA)
}

static DISPLAY_LISTS_DATA: &[u8] = include_bytes_align!(32, "../../../build/display_lists.dat");

#[repr(C)]
struct DisplayListsByClusterTexturePlaneEntry {
    by_texture_plane_start_index: usize,
    by_texture_plane_end_index: usize,
}

unsafe impl FullyOccupied for DisplayListsByClusterTexturePlaneEntry {}

impl DisplayListsByClusterTexturePlaneEntry {
    fn iter_display_lists(&self) -> impl Iterator<Item = TexturePlaneDisplayList> {
        display_lists_by_texture_plane()
            [self.by_texture_plane_start_index..self.by_texture_plane_end_index]
            .iter()
            .map(|entry| entry.iter_display_lists())
            .flatten()
    }
}

#[repr(C)]
struct DisplayListsByTexturePlaneEntry {
    texture_index: usize,
    by_plane_start_index: usize,
    by_plane_end_index: usize,
}

unsafe impl FullyOccupied for DisplayListsByTexturePlaneEntry {}

impl DisplayListsByTexturePlaneEntry {
    fn iter_display_lists(&self) -> impl Iterator<Item = TexturePlaneDisplayList> {
        let texture_index = self.texture_index;
        display_lists_by_plane()[self.by_plane_start_index..self.by_plane_end_index]
            .iter()
            .map(move |entry| entry.get_display_list(texture_index))
    }
}

#[repr(C)]
struct DisplayListsByPlaneEntry {
    plane_index: u16,
    _padding: u16,
    reflect_front_paraboloid: [[f32; 4]; 3],
    reflect_back_paraboloid: [[f32; 4]; 3],
    reflect_paraboloid_z: [[f32; 4]; 3],
    display_list_start_offset: usize,
    display_list_end_offset: usize,
}

unsafe impl FullyOccupied for DisplayListsByPlaneEntry {}

impl DisplayListsByPlaneEntry {
    fn get_display_list(&self, texture_index: usize) -> TexturePlaneDisplayList {
        TexturePlaneDisplayList {
            texture_index,
            plane_index: self.plane_index,
            reflect_front_paraboloid: &self.reflect_front_paraboloid,
            reflect_back_paraboloid: &self.reflect_back_paraboloid,
            reflect_paraboloid_z: &self.reflect_paraboloid_z,
            display_list: &DISPLAY_LISTS_DATA
                [self.display_list_start_offset..self.display_list_end_offset],
        }
    }
}
