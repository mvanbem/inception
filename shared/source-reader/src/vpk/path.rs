use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter};

#[cfg(feature = "quickcheck")]
use quickcheck::Arbitrary;

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

#[cfg(feature = "quickcheck")]
impl Arbitrary for VpkPath {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        fn append_four_letters(g: &mut quickcheck::Gen, s: &mut String) {
            for _ in 0..4 {
                s.push((b'a' + u8::arbitrary(g) % 26) as char);
            }
        }

        let mut name = String::new();
        append_four_letters(g, &mut name);
        name.push('/');
        append_four_letters(g, &mut name);

        let mut prefix = String::new();
        append_four_letters(g, &mut prefix);

        let mut extension = String::new();
        append_four_letters(g, &mut extension);

        Self::new_with_prefix_and_extension(&name, &prefix, &extension)
    }
}

impl Debug for VpkPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl Display for VpkPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl Ord for VpkPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for VpkPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.path.partial_cmp(&other.path)
    }
}
