use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::Result;

use crate::file::FileLoader;
use crate::vpk::path::VpkPath;

pub struct DirectoryLoader {
    path: PathBuf,
}

impl DirectoryLoader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl FileLoader for DirectoryLoader {
    fn load_file(&self, path: &VpkPath) -> Result<Option<Vec<u8>>> {
        let combined_path = self.path.join(path.as_canonical_path().as_str());
        match std::fs::read(&combined_path) {
            Ok(data) => Ok(Some(data)),
            Err(e) => match e.kind() {
                ErrorKind::NotFound => Ok(None),
                _ => Err(e.into()),
            },
        }
    }
}
