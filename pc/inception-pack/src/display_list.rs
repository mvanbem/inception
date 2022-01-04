use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub struct DisplayListBuilder {
    data: Vec<u8>,
    start_offset: u32,
}

impl DisplayListBuilder {
    pub const QUADS: u8 = 0x80;
    pub const TRIANGLES: u8 = 0x90;

    pub fn new(primitive: u8) -> Self {
        let mut data = Vec::new();
        data.push(primitive);
        data.write_u16::<BigEndian>(0).unwrap();
        Self {
            data,
            start_offset: 0,
        }
    }

    fn read_primitive(&self) -> u8 {
        self.data[self.start_offset as usize]
    }

    fn read_count(&self) -> u16 {
        (&self.data[self.start_offset as usize + 1..])
            .read_u16::<BigEndian>()
            .unwrap()
    }

    fn write_count(&mut self, count: u16) {
        (&mut self.data[self.start_offset as usize + 1..])
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
            let new_start_offset = u32::try_from(self.data.len()).unwrap();
            self.data.push(self.read_primitive());
            self.data.write_u16::<BigEndian>(count).unwrap();
            self.data.extend_from_slice(data);
            self.start_offset = new_start_offset;
        }
    }

    pub fn build(mut self) -> Vec<u8> {
        if self.data.len() == 3 {
            // Nothing was written.
            Vec::new()
        } else {
            // Pad to a 32-byte boundary with NOP bytes.
            while (self.data.len() & 31) != 0 {
                self.data.push(0x00);
            }
            self.data
        }
    }
}
