#![cfg_attr(not(feature = "std"), no_std)]

use crate::hashable_float::HashableMat;

extern crate alloc;

pub mod bytecode;
pub mod hashable_float;
pub mod map_data;
pub mod pipeline_state;
pub mod shader;
pub mod vertex_format;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Plane {
    pub normal: HashableMat<3, 1>,
}
