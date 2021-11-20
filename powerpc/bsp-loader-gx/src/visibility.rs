#[derive(Clone, Copy)]
pub struct Visibility {
    data: *const u8,
}

impl Visibility {
    pub fn new(data: *const u8) -> Self {
        Self { data }
    }

    pub unsafe fn num_clusters(self) -> usize {
        unsafe { *(self.data as *const u32) as usize }
    }

    pub unsafe fn get_cluster(self, index: ClusterIndex) -> VisibilityBitmap {
        unsafe {
            let num_clusters = self.num_clusters();
            assert!(index.0 < num_clusters);
            let pvs_byte_ofs = *(self.data as *const u32).offset(index.0 as isize + 1);
            VisibilityBitmap {
                data: self.data.offset(pvs_byte_ofs as isize),
                num_clusters,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClusterIndex(pub usize);

#[derive(Clone, Copy)]
pub struct VisibilityBitmap {
    data: *const u8,
    num_clusters: usize,
}

impl VisibilityBitmap {
    pub fn iter_visible_clusters(self) -> impl Iterator<Item = ClusterIndex> {
        VisibilityBitmapIter {
            data: self.data,
            cluster_index: 0,
            num_clusters: self.num_clusters,
            current_byte: 0,
            current_bit: 0,
        }
    }
}

struct VisibilityBitmapIter {
    data: *const u8,
    cluster_index: usize,
    num_clusters: usize,
    current_byte: u8,
    current_bit: u8,
}

impl Iterator for VisibilityBitmapIter {
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
            let b = unsafe { *self.data };
            self.data = unsafe { self.data.offset(1) };
            match b {
                0 => {
                    let run_len = unsafe { *self.data } as usize;
                    self.data = unsafe{self.data.offset(1)};
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
