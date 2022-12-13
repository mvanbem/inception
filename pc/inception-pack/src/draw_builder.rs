use gx::display_list::{Command, DisplayList, GxPrimitive};

#[derive(Clone)]
pub struct DrawBuilder {
    primitive: GxPrimitive,
    vertex_format: u8,
    vertex_count: usize,
    vertex_data: Vec<u8>,
}

impl DrawBuilder {
    pub fn new(primitive: GxPrimitive, vertex_format: u8) -> Self {
        Self {
            primitive,
            vertex_format,
            vertex_count: 0,
            vertex_data: Vec::new(),
        }
    }

    pub fn emit_vertices(&mut self, count: usize, data: &[u8]) {
        self.vertex_count += count;
        self.vertex_data.extend_from_slice(data);
    }

    pub fn build(self) -> DisplayList {
        if self.vertex_count == 0 {
            DisplayList::default()
        } else {
            DisplayList {
                commands: vec![Command::Draw {
                    primitive: self.primitive,
                    vertex_format: self.vertex_format,
                    vertex_count: self.vertex_count.try_into().unwrap(),
                    vertex_data: self.vertex_data,
                }],
            }
        }
    }
}
