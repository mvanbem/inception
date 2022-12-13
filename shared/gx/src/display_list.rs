use std::io::{self, Write};

use byteorder::{BigEndian, WriteBytesExt};

use crate::bp::{BpInterleavedTexReg, BpTexImageRegD};
use crate::cp::CpReg;
use crate::xf::XfReg;

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub commands: Vec<Command>,
}

impl DisplayList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.commands.iter().map(Command::len).sum()
    }

    pub fn pad_to_alignment(&mut self) {
        let mut len = self.len();
        while (len & 31) != 0 {
            self.commands.push(Command::Nop);
            len += 1;
        }
        debug_assert_eq!(self.len() & 31, 0);
    }

    pub fn write_to<W: Write>(
        &self,
        w: &mut W,
        mut emit_reference: impl FnMut(&W, Reference),
    ) -> io::Result<()> {
        for command in &self.commands {
            command.write_to(w, &mut emit_reference)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Nop,
    WriteBpReg {
        packed_addr_and_value: u32,
        reference: Option<Reference>,
    },
    WriteCpReg {
        addr: u8,
        value: u32,
    },
    WriteXfReg {
        addr: u16,
        values: Vec<u32>,
    },
    Draw {
        primitive: GxPrimitive,
        vertex_format: u8,
        vertex_count: u16,
        vertex_data: Vec<u8>,
    },
}

impl Command {
    pub fn write_bp_tex_image_reg_d_reference(image: u8, texture_id: u16) -> Option<Self> {
        Some(Self::WriteBpReg {
            packed_addr_and_value: BpTexImageRegD::new()
                .with_addr(BpTexImageRegD::addr_for_image(image)?)
                .into(),
            reference: Some(Reference::Texture(texture_id)),
        })
    }

    pub fn write_cp_reg<R: CpReg>(reg: R, value: R::T) -> Self {
        Self::WriteCpReg {
            addr: reg.addr(),
            value: value.into(),
        }
    }

    pub fn write_xf_reg<R: XfReg>(reg: R, value: R::T) -> Self {
        Self::WriteXfReg {
            addr: reg.addr(),
            values: vec![value.into()],
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Command::Nop => 1,
            Command::WriteBpReg { .. } => 5,
            Command::WriteCpReg { .. } => 6,
            Command::WriteXfReg { values, .. } => 5 + 4 * values.len(),
            Command::Draw { vertex_data, .. } => 3 + vertex_data.len(),
        }
    }

    pub fn write_to<W: Write>(
        &self,
        w: &mut W,
        mut emit_reference: impl FnMut(&W, Reference),
    ) -> io::Result<()> {
        match self {
            Command::Nop => {
                w.write_u8(0x00)?;
            }
            Command::WriteBpReg {
                packed_addr_and_value,
                reference,
            } => {
                w.write_u8(0x61)?;
                if let Some(reference) = reference {
                    emit_reference(w, *reference);
                }
                w.write_u32::<BigEndian>(*packed_addr_and_value)?;
            }
            Command::WriteCpReg { addr, value } => {
                w.write_u8(0x08)?;
                w.write_u8(*addr)?;
                w.write_u32::<BigEndian>(*value)?;
            }
            Command::WriteXfReg { addr, values } => {
                assert!(values.len() <= 16);
                w.write_u8(0x10)?;
                w.write_u16::<BigEndian>(values.len() as u16 - 1)?;
                w.write_u16::<BigEndian>(*addr)?;
                for &value in values {
                    w.write_u32::<BigEndian>(value)?;
                }
            }
            Self::Draw {
                primitive,
                vertex_format,
                vertex_count,
                vertex_data,
            } => {
                assert!(*vertex_format <= 7);
                w.write_u8(primitive.as_u8() | vertex_format)?;
                w.write_u16::<BigEndian>(*vertex_count)?;
                w.write_all(vertex_data)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reference {
    Texture(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[cfg(test)]
mod tests {

    use crate::common::{
        GeometryMatrix, MatrixRegA, MatrixRegB, PostTransformMatrix, TextureMatrix,
    };
    use crate::cp::{CpMatrixRegA, CpMatrixRegB};
    use crate::xf::{TexCoordGenRegB, XfMatrixRegA, XfMatrixRegB, XfTexCoordGenRegB};

    use super::*;

    #[test]
    fn cp_matrices() {
        let mut dl = DisplayList::new();
        dl.commands.push(Command::write_cp_reg(
            CpMatrixRegA,
            MatrixRegA::new()
                .with_geometry(GeometryMatrix::PNMTX1)
                .with_tex0(TextureMatrix::TEXMTX2)
                .with_tex1(TextureMatrix::TEXMTX3)
                .with_tex2(TextureMatrix::TEXMTX4)
                .with_tex3(TextureMatrix::TEXMTX5),
        ));
        dl.commands.push(Command::write_cp_reg(
            CpMatrixRegB,
            MatrixRegB::new()
                .with_tex4(TextureMatrix::TEXMTX6)
                .with_tex5(TextureMatrix::TEXMTX7)
                .with_tex6(TextureMatrix::TEXMTX8)
                .with_tex7(TextureMatrix::TEXMTX9),
        ));
        let mut data = Vec::new();
        dl.write_to(&mut data, |_, _| panic!()).unwrap();
        assert_eq!(
            data,
            &[
                0x08,
                0x30,
                0b00_101101,
                0b101010_10,
                0b0111_1001,
                0b00_000011,
                0x08,
                0x40,
                0b00000000,
                0b111001_11,
                0b0110_1100,
                0b11_110000,
            ]
        );
    }

    #[test]
    fn xf_matrices() {
        let mut dl = DisplayList::new();
        dl.commands.push(Command::write_xf_reg(
            XfMatrixRegA,
            MatrixRegA::new()
                .with_geometry(GeometryMatrix::PNMTX1)
                .with_tex0(TextureMatrix::TEXMTX2)
                .with_tex1(TextureMatrix::TEXMTX3)
                .with_tex2(TextureMatrix::TEXMTX4)
                .with_tex3(TextureMatrix::TEXMTX5),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfMatrixRegB,
            MatrixRegB::new()
                .with_tex4(TextureMatrix::TEXMTX6)
                .with_tex5(TextureMatrix::TEXMTX7)
                .with_tex6(TextureMatrix::TEXMTX8)
                .with_tex7(TextureMatrix::TEXMTX9),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX0,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX9)
                .with_normalize(false),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX1,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX8)
                .with_normalize(false),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX2,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX7)
                .with_normalize(false),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX3,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX6)
                .with_normalize(true),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX4,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX5)
                .with_normalize(true),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX5,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX4)
                .with_normalize(true),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX6,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX3)
                .with_normalize(false),
        ));
        dl.commands.push(Command::write_xf_reg(
            XfTexCoordGenRegB::TEX7,
            TexCoordGenRegB::new()
                .with_post_transform_matrix(PostTransformMatrix::DTTMTX2)
                .with_normalize(false),
        ));
        let mut data = Vec::new();
        dl.write_to(&mut data, |_, _| panic!()).unwrap();
        assert_eq!(
            data,
            &[
                0x10,
                0x00,
                0x00,
                0x10,
                0x18,
                0b00_101101,
                0b101010_10,
                0b0111_1001,
                0b00_000011,
                0x10,
                0x00,
                0x00,
                0x10,
                0x19,
                0b00000000,
                0b111001_11,
                0b0110_1100,
                0b11_110000,
                0x10,
                0x00,
                0x00,
                0x10,
                0x50,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00011011,
                0x10,
                0x00,
                0x00,
                0x10,
                0x51,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00011000,
                0x10,
                0x00,
                0x00,
                0x10,
                0x52,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00010101,
                0x10,
                0x00,
                0x00,
                0x10,
                0x53,
                0b00000000,
                0b00000000,
                0b00000001,
                0b00010010,
                0x10,
                0x00,
                0x00,
                0x10,
                0x54,
                0b00000000,
                0b00000000,
                0b00000001,
                0b00001111,
                0x10,
                0x00,
                0x00,
                0x10,
                0x55,
                0b00000000,
                0b00000000,
                0b00000001,
                0b00001100,
                0x10,
                0x00,
                0x00,
                0x10,
                0x56,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00001001,
                0x10,
                0x00,
                0x00,
                0x10,
                0x57,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000110,
            ]
        );
    }

    #[test]
    fn write_bp_tex_image_reg_d_reference() {
        let mut dl = DisplayList::new();
        dl.commands
            .push(Command::write_bp_tex_image_reg_d_reference(3, 5).unwrap());
        let mut data = Vec::new();
        let mut references = Vec::new();
        dl.write_to(&mut data, |data, reference| {
            references.push((data.len(), reference));
        })
        .unwrap();

        assert_eq!(data, &[0x61, 0x97, 0x00, 0x00, 0x00]);
        assert_eq!(references, &[(5, Reference::Texture(5))]);
    }
}
