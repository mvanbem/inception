use fully_occupied::{extract, extract_slice, slice_as_bytes, FullyOccupied};

pub fn get_cluster_geometry(cluster: u16) -> &'static ClusterGeometry {
    &cluster_geometry_table()[cluster as usize]
}

static CLUSTER_GEOMETRY_TABLE_DATA: &[u8] =
    include_bytes_align!(4, "../../../build/cluster_geometry_table.dat");
fn cluster_geometry_table() -> &'static [ClusterGeometry] {
    extract_slice(CLUSTER_GEOMETRY_TABLE_DATA)
}

static CLUSTER_GEOMETRY_BYTE_CODE_DATA: &[u8] =
    include_bytes_align!(4, "../../../build/cluster_geometry_byte_code.dat");
fn cluster_geometry_byte_code() -> &'static [u32] {
    extract_slice(CLUSTER_GEOMETRY_BYTE_CODE_DATA)
}

static DISPLAY_LISTS_DATA: &[u8] = include_bytes_align!(32, "../../../build/display_lists.dat");

#[repr(C)]
pub struct ClusterGeometry {
    byte_code_index_ranges: [[usize; 2]; 18],
}

unsafe impl FullyOccupied for ClusterGeometry {}

impl ClusterGeometry {
    pub fn iter_display_lists(&self, pass: usize) -> impl Iterator<Item = ByteCodeEntry> {
        ByteCodeReader(
            &cluster_geometry_byte_code()
                [self.byte_code_index_ranges[pass][0]..self.byte_code_index_ranges[pass][1]],
        )
    }
}

pub enum ByteCodeEntry<'a> {
    Draw {
        display_list: &'static [u8],
    },
    SetPlane {
        texture_matrix: &'a [[f32; 4]; 3],
    },
    SetBaseTexture {
        base_texture_index: u16,
    },
    SetEnvMapTexture {
        env_map_texture_index: u16,
    },
    SetEnvMapTint {
        r: u8,
        g: u8,
        b: u8,
    },
    SetAlpha {
        test_threshold: Option<u8>,
        blend: bool,
    },
    SetAuxTexture {
        aux_texture_index: u16,
    },
}

struct ByteCodeReader<'a>(&'a [u32]);

impl<'a> Iterator for ByteCodeReader<'a> {
    type Item = ByteCodeEntry<'a>;

    fn next(&mut self) -> Option<ByteCodeEntry<'a>> {
        let op = self.0.get(0).copied()? >> 24;
        match op {
            0x00 => {
                let start_offset = (self.0[0] & 0x00ffffff) as usize;
                let end_offset = self.0[1] as usize;
                self.0 = &self.0[2..];
                Some(ByteCodeEntry::Draw {
                    display_list: &DISPLAY_LISTS_DATA[start_offset..end_offset],
                })
            }
            0x01 => {
                let texture_matrix = extract(slice_as_bytes(&self.0[1..13]));
                self.0 = &self.0[13..];
                Some(ByteCodeEntry::SetPlane { texture_matrix })
            }
            0x02 => {
                let base_texture_index = self.0[0] as u16;
                self.0 = &self.0[1..];
                Some(ByteCodeEntry::SetBaseTexture { base_texture_index })
            }
            0x03 => {
                let env_map_texture_index = self.0[0] as u16;
                self.0 = &self.0[1..];
                Some(ByteCodeEntry::SetEnvMapTexture {
                    env_map_texture_index,
                })
            }
            0x04 => {
                let r = (self.0[0] >> 16) as u8;
                let g = (self.0[0] >> 8) as u8;
                let b = self.0[0] as u8;
                self.0 = &self.0[1..];
                Some(ByteCodeEntry::SetEnvMapTint { r, g, b })
            }
            0x05 => {
                let test = (self.0[0] >> 16) as u8 != 0;
                let threshold = (self.0[0] >> 8) as u8;
                let blend = self.0[0] as u8 != 0;
                self.0 = &self.0[1..];
                Some(ByteCodeEntry::SetAlpha {
                    test_threshold: if test { Some(threshold) } else { None },
                    blend,
                })
            }
            0x06 => {
                let aux_texture_index = self.0[0] as u16;
                self.0 = &self.0[1..];
                Some(ByteCodeEntry::SetAuxTexture { aux_texture_index })
            }
            _ => panic!("unexpected geometry op: 0x{:02x}", op),
        }
    }
}
