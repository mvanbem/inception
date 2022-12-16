use alloc::vec::Vec;

pub enum BytecodeOp {
    Draw {
        display_list_offset: u32,
        display_list_size: u32,
    },
    SetVertexDesc {
        attr_list_offset: u32,
    },
    SetAlphaCompare {
        z_comp_before_tex: u8,
        compare_type: u8,
        reference: u8,
    },
    SetFaceIndex {
        face_index: u16,
    },
}

impl BytecodeOp {
    pub const ALPHA_COMPARE_TYPE_GEQUAL: u8 = 6;
    pub const ALPHA_COMPARE_TYPE_ALWAYS: u8 = 7;

    pub fn append_to(&self, bytecode: &mut Vec<u32>) {
        match self {
            &Self::Draw {
                display_list_offset,
                display_list_size,
            } => {
                assert_eq!(display_list_offset & 0xff000000, 0);
                bytecode.push(display_list_offset);
                bytecode.push(display_list_size);
            }
            &Self::SetVertexDesc { attr_list_offset } => {
                assert_eq!(attr_list_offset & 0xff000000, 0);
                bytecode.push(0x01000000 | attr_list_offset);
            }
            &Self::SetAlphaCompare {
                z_comp_before_tex,
                compare_type,
                reference,
            } => {
                bytecode.push(
                    0x02000000
                        | (z_comp_before_tex as u32) << 16
                        | (compare_type as u32) << 8
                        | reference as u32,
                );
            }
            &Self::SetFaceIndex { face_index } => {
                bytecode.push(0x03000000 | face_index as u32);
            }
        }
    }
}

pub struct BytecodeReader<'a>(&'a [u32]);

impl<'a> BytecodeReader<'a> {
    pub fn new(words: &'a [u32]) -> Self {
        Self(words)
    }
}

impl<'a> Iterator for BytecodeReader<'a> {
    type Item = BytecodeOp;

    fn next(&mut self) -> Option<BytecodeOp> {
        let op = self.0.get(0).copied()? >> 24;
        match op {
            0x00 => {
                let display_list_offset = self.0[0] & 0x00ffffff;
                let display_list_size = self.0[1];
                self.0 = &self.0[2..];
                Some(BytecodeOp::Draw {
                    display_list_offset,
                    display_list_size,
                })
            }
            0x01 => {
                let attr_list_offset = self.0[0] & 0x00ffffff;
                self.0 = &self.0[1..];
                Some(BytecodeOp::SetVertexDesc { attr_list_offset })
            }
            0x02 => {
                let z_comp_before_tex = (self.0[0] >> 16) as u8;
                let compare_type = (self.0[0] >> 8) as u8;
                let reference = self.0[0] as u8;
                self.0 = &self.0[1..];
                Some(BytecodeOp::SetAlphaCompare {
                    z_comp_before_tex,
                    compare_type,
                    reference,
                })
            }
            0x03 => {
                let face_index = self.0[0] as u16;
                self.0 = &self.0[1..];
                Some(BytecodeOp::SetFaceIndex { face_index })
            }
            _ => panic!("unexpected geometry op: 0x{:02x}", op),
        }
    }
}
