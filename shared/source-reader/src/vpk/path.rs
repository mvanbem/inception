use std::fmt::{self, Display, Formatter};

use crate::file::canonical_path::{display_canonical, CanonicalPath, CanonicalPathBuf};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VpkPath {
    path: CanonicalPathBuf,
    last_slash_index: Option<usize>,
    last_period_index: usize,
}

impl VpkPath {
    pub fn new_with_prefix_and_extension(name: &str, mut prefix: &str, extension: &str) -> Self {
        if name.starts_with(prefix) {
            prefix = "";
        }

        let path = CanonicalPathBuf::from_string(format!(
            "{}{}{}{}{}",
            display_canonical(prefix),
            if !prefix.is_empty() { "/" } else { "" },
            display_canonical(name),
            if !name.contains('.') { "." } else { "" },
            display_canonical(if !name.contains('.') { extension } else { "" }),
        ))
        .unwrap();
        let last_slash_index = path.as_str().rfind('/');
        let last_period_index = path.as_str().rfind('.').unwrap();
        if let Some(last_slash_index) = last_slash_index {
            assert!(last_slash_index < last_period_index);
        }
        Self {
            path,
            last_slash_index,
            last_period_index,
        }
    }

    pub fn as_canonical_path(&self) -> &CanonicalPath {
        &self.path
    }

    pub fn parent(&self) -> &CanonicalPath {
        match self.last_slash_index {
            Some(last_slash_index) => &self.path[..last_slash_index],
            None => Default::default(),
        }
    }

    pub fn file_stem(&self) -> &CanonicalPath {
        match self.last_slash_index {
            Some(last_slash_index) => &self.path[last_slash_index + 1..self.last_period_index],
            None => &self.path[..self.last_period_index],
        }
    }

    pub fn extension(&self) -> &CanonicalPath {
        &self.path[self.last_period_index + 1..]
    }
}

impl Display for VpkPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}
