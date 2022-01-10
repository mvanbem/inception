use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use inception_render_common::map_data::MapData;

use crate::loader::Loader;

#[repr(align(32))]
struct Align32Bytes;

static MAP_DATA: &[u8] = include_bytes_align_as!(Align32Bytes, "../../../../build/map.dat");

pub struct EmbeddedLoader;

impl Loader for EmbeddedLoader {
    type Params = ();
    type Data = &'static [u8];

    fn new(_: ()) -> Self {
        Self
    }

    fn maps(&mut self) -> Vec<String> {
        vec!["embedded".to_string()]
    }

    fn load_map(&mut self, _map: &str) -> MapData<Self::Data> {
        unsafe { MapData::new(MAP_DATA) }
    }
}
