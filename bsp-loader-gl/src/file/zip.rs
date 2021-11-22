use std::cell::RefCell;
use std::io::{Read, Seek};

use anyhow::Result;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::file::FileLoader;
use crate::vpk::path::VpkPath;

pub struct ZipArchiveLoader<R> {
    archive: RefCell<ZipArchive<R>>,
}

impl<R> ZipArchiveLoader<R> {
    pub fn new(archive: ZipArchive<R>) -> Self {
        Self {
            archive: RefCell::new(archive),
        }
    }
}

impl<R: Read + Seek> FileLoader for ZipArchiveLoader<R> {
    fn load_file(&self, path: &VpkPath) -> Result<Option<Vec<u8>>> {
        match self
            .archive
            .borrow_mut()
            .by_name(path.as_canonical_path().as_str())
        {
            Ok(mut file) => {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                Ok(Some(buf))
            }
            Err(ZipError::FileNotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
