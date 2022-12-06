use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GxPrimitive {
    Quads,
    Triangles,
}

impl GxPrimitive {
    fn as_u8(self) -> u8 {
        match self {
            GxPrimitive::Quads => 0x80,
            GxPrimitive::Triangles => 0x90,
        }
    }
}

#[derive(Clone, Default)]
pub struct DisplayListBuilder {
    data: Vec<u8>,
}

impl DisplayListBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(mut self) -> Vec<u8> {
        // Pad to a 32-byte boundary with NOP bytes.
        while (self.data.len() & 31) != 0 {
            self.data.push(0x00);
        }
        self.data
    }

    pub fn into_draw(self, primitive: GxPrimitive) -> DisplayListBuilderDraw {
        DisplayListBuilderDraw::new(self.data, primitive)
    }
}

#[derive(Clone)]
pub struct DisplayListBuilderDraw {
    data: Vec<u8>,
    start_offset: usize,
}

impl DisplayListBuilderDraw {
    fn new(mut data: Vec<u8>, primitive: GxPrimitive) -> Self {
        let start_offset = data.len();
        data.push(primitive.as_u8());
        // Write a placeholder vertex count. It will be overwritten when the draw command is
        // complete.
        data.write_u16::<BigEndian>(0).unwrap();
        Self { data, start_offset }
    }

    fn read_primitive(&self) -> u8 {
        self.data[self.start_offset as usize]
    }

    fn read_count(&self) -> u16 {
        (&self.data[self.start_offset + 1..])
            .read_u16::<BigEndian>()
            .unwrap()
    }

    fn write_count(&mut self, count: u16) {
        (&mut self.data[self.start_offset + 1..])
            .write_u16::<BigEndian>(count)
            .unwrap()
    }

    pub fn emit_vertices(&mut self, count: u16, data: &[u8]) {
        if let Some(new_count) = self.read_count().checked_add(count) {
            // The command will not overflow. Append the data and update the count.
            self.data.extend_from_slice(data);
            self.write_count(new_count);
        } else {
            // The command would overflow. Start a new command and append the data.
            let new_start_offset = self.data.len();
            self.data.push(self.read_primitive());
            self.data.write_u16::<BigEndian>(count).unwrap();
            self.data.extend_from_slice(data);
            self.start_offset = new_start_offset;
        }
    }

    pub fn finish(mut self) -> DisplayListBuilder {
        if self.data.len() == self.start_offset + 3 {
            // Nothing was written. Remove the command byte and placeholder length.
            self.data.truncate(self.start_offset);
        }
        DisplayListBuilder { data: self.data }
    }
}
