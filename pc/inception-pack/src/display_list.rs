use std::num::NonZeroU32;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DisplayListOffset {
    pub offset: u32,
    pub size: NonZeroU32,
}

#[derive(Default)]
pub struct DisplayListBuilder {
    data: Vec<u8>,
}

impl DisplayListBuilder {
    pub const TRIANGLES: u8 = 0x90;

    pub fn build_batch(&mut self, primitive: u8) -> BatchBuilder {
        BatchBuilder::new(&mut self.data, primitive)
    }

    /// Pad to a 32-byte boundary with NOP bytes.
    fn pad(&mut self) {
        while (self.data.len() & 31) != 0 {
            self.data.push(0x00);
        }
    }

    pub fn build(mut self) -> Vec<u8> {
        self.pad();
        self.data
    }
}

pub struct BatchBuilder<'a> {
    data: &'a mut Vec<u8>,
    start_offset: u32,
}

impl<'a> BatchBuilder<'a> {
    fn new(data: &'a mut Vec<u8>, primitive: u8) -> Self {
        let start_offset = u32::try_from(data.len()).unwrap();
        data.push(primitive);
        data.write_u16::<BigEndian>(0).unwrap();
        Self { data, start_offset }
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
}

impl<'a> Drop for BatchBuilder<'a> {
    fn drop(&mut self) {
        // If no vertices were written, back out the command and length.
        if self.read_count() == 0 {
            self.data.resize(self.data.len() - 3, 0);
        }
    }
}
