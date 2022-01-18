use crate::iso9660::DiscReader;
use crate::loader::Loader;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use inception_render_common::map_data::MapData;
use ogc_sys::GlobalAlign32;

pub struct DvdIsoLoader {
    disc_reader: DiscReader,
}

impl Loader for DvdIsoLoader {
    type Params = ();
    type Data = Vec<u8, GlobalAlign32>;

    fn new(_: ()) -> Self {
        Self {
            disc_reader: DiscReader::new(),
        }
    }

    fn maps(&mut self) -> Vec<String> {
        let mut maps = Vec::new();

        self.disc_reader.list_directory("maps", |name| {
            maps.push(name.to_string());
        });

        maps
    }

    fn load_map(&mut self, map: &str) -> MapData<Self::Data> {
        let data = self.disc_reader.read_file(&format!("maps/{}", map));
        unsafe { MapData::new(data) }
    }
}
