use std::rc::Rc;

use anyhow::{bail, Result};

use crate::file::vpk::path::VpkPath;

pub mod canonical_path;
pub mod vpk;
pub mod zip;

pub trait FileLoader {
    fn load_file(&self, path: &VpkPath) -> Result<Option<Vec<u8>>>;
}

pub struct FallbackFileLoader<'a> {
    loaders: Vec<Box<Rc<dyn FileLoader + 'a>>>,
}

impl<'a> FallbackFileLoader<'a> {
    pub fn new(loaders: Vec<Box<Rc<dyn FileLoader + 'a>>>) -> Self {
        Self { loaders }
    }
}

impl<'a> FileLoader for FallbackFileLoader<'a> {
    fn load_file(&self, path: &VpkPath) -> Result<Option<Vec<u8>>> {
        for loader in &self.loaders {
            match loader.load_file(path)? {
                Some(data) => return Ok(Some(data)),
                None => (),
            }
        }
        bail!("file not found: {}", path)
    }
}
