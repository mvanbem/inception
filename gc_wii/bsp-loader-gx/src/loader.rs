use core::ops::Deref;

use alloc::string::String;
use alloc::vec::Vec;
use inception_render_common::map_data::MapData;

pub mod dvd_gcm_loader;
pub mod dvd_iso_loader;
#[cfg(feature = "embedded_loader")]
pub mod embedded_loader;
pub mod ftp_loader;

pub trait Loader: Sized {
    type Params<'a>;
    type Data: Deref<Target = [u8]>;

    /// This might do a lot of I/O.
    fn new(params: Self::Params<'_>) -> Self;

    /// This might do a lot of I/O.
    fn maps(&mut self) -> Vec<String>;

    /// This might do a lot of I/O.
    fn load_map(&mut self, map: &str) -> MapData<Self::Data>;
}
