#[cfg(feature = "std")]
use std::io::{self, Write};

use alloc::vec::Vec;
#[cfg(feature = "std")]
use byteorder::WriteBytesExt;

use crate::hashable_float::HashableMat3x4;
use crate::shader::Shader;

/// Uniquely identifies a graphics processor state needed to render a batch.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PipelineState {
    pub shader: Shader,

    /// Reflection matrix. Required for env map shaders.
    pub reflection_matrix: HashableMat3x4,

    /// Which vertex attributes are enabled and their input types.
    pub vertex_desc: VertexDesc,

    /// Texture ID to bind to GX_TEXTURE1. Typically used for a base color, sometimes with an alpha
    /// channel for opacity or an env map mask.
    pub base_texture_id: u16,

    /// Texture ID to bind to GX_TEXTURE2. Typically used for opacity and/or an env map mask.
    pub aux_texture_id: Option<u16>,

    /// Texture ID to bind to GX_TEXTURE3. Reserved for an env map.
    pub env_texture_id: Option<u16>,

    /// Color to bind to konstant register 0. Typically used for an env map tint.
    pub env_map_tint: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum VertexAttribute {
    Pos = 9,
    Nrm = 10,
    Clr0 = 11,
    Clr1 = 12,
    Tex0 = 13,
    Tex1 = 14,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum VertexInput {
    Direct = 1,
    Index8 = 2,
    Index16 = 3,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexDesc {
    desc: Vec<(VertexAttribute, VertexInput)>,
}

impl VertexDesc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, attribute: VertexAttribute, input: VertexInput) {
        match self
            .desc
            .binary_search_by_key(&attribute, |&(attribute, _)| attribute)
        {
            Ok(index) => self.desc[index].1 = input,
            Err(index) => self.desc.insert(index, (attribute, input)),
        }
    }

    #[cfg(feature = "std")]
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        for &(attribute, input) in &self.desc {
            w.write_u8(attribute as u8)?;
            w.write_u8(input as u8)?;
        }
        Ok(())
    }
}
