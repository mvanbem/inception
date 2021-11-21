use std::borrow::Borrow;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;
use std::mem;
use std::ops::{Deref, Index};

fn canonical(c: char) -> char {
    if c == '\\' {
        '/'
    } else {
        c.to_ascii_lowercase()
    }
}

fn is_canonical(c: char) -> bool {
    !c.is_ascii_uppercase() && c != '\\'
}

struct DisplayCanonical<'a> {
    inner: &'a str,
}

impl<'a> Display for DisplayCanonical<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for c in self.inner.chars().map(canonical) {
            write!(f, "{}", c)?;
        }
        Ok(())
    }
}

pub fn display_canonical(inner: &str) -> impl Display + '_ {
    DisplayCanonical { inner }
}

// A path that is in canonical form with respect to letter case and slash orientation.
//
// # Canonical form
// All ASCII letters are lowercase. Forward slashes may be present, but not backslashes.
#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CanonicalPath {
    inner: str,
}

impl CanonicalPath {
    pub fn from_str(value: &str) -> Option<&Self> {
        if value.chars().all(is_canonical) {
            // SAFETY: We just checked.
            Some(unsafe { Self::from_str_unchecked(value) })
        } else {
            None
        }
    }

    /// # Safety
    /// The string must be in canonical form.
    pub unsafe fn from_str_unchecked(value: &str) -> &Self {
        // SAFETY: `Self` is `repr(transparent)`, so it has the same memory layout as `str`.
        unsafe { mem::transmute(value) }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl AsRef<str> for CanonicalPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Debug for CanonicalPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", &self.inner)
    }
}

impl Display for CanonicalPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

impl<Idx> Index<Idx> for CanonicalPath
where
    str: Index<Idx, Output = str>,
{
    type Output = Self;

    fn index(&self, index: Idx) -> &Self {
        // SAFETY: The whole string is in canonical form, so all substrings are as well.
        unsafe { Self::from_str_unchecked(self.inner.index(index)) }
    }
}

impl ToOwned for CanonicalPath {
    type Owned = CanonicalPathBuf;

    fn to_owned(&self) -> CanonicalPathBuf {
        // SAFETY: Safe operations on `Self` maintain canonical form.
        unsafe { CanonicalPathBuf::from_string_unchecked(self.inner.to_string()) }
    }
}

impl Default for &CanonicalPath {
    fn default() -> Self {
        // SAFETY: The empty string is in canonical form.
        unsafe { CanonicalPath::from_str_unchecked("") }
    }
}

/// An owned [`CanonicalPath`].
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct CanonicalPathBuf {
    inner: String,
}

impl CanonicalPathBuf {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_string(value: String) -> Option<Self> {
        if value.chars().all(is_canonical) {
            Some(Self { inner: value })
        } else {
            None
        }
    }

    pub fn from_str_canonicalize(value: &str) -> Self {
        match CanonicalPath::from_str(value) {
            Some(x) => x.to_owned(),
            // SAFETY: The string is constructed in canonical form.
            None => unsafe { Self::from_string_unchecked(value.chars().map(canonical).collect()) },
        }
    }

    /// # Safety
    /// The string must be in canonical form.
    pub unsafe fn from_string_unchecked(value: String) -> Self {
        Self { inner: value }
    }
}

impl AsRef<CanonicalPath> for CanonicalPathBuf {
    fn as_ref(&self) -> &CanonicalPath {
        &*self
    }
}

impl Borrow<CanonicalPath> for CanonicalPathBuf {
    fn borrow(&self) -> &CanonicalPath {
        &*self
    }
}

impl Debug for CanonicalPathBuf {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", &self.inner)
    }
}

impl Deref for CanonicalPathBuf {
    type Target = CanonicalPath;

    fn deref(&self) -> &CanonicalPath {
        // SAFETY: Safe operations on `Self` maintain canonical form.
        unsafe { CanonicalPath::from_str_unchecked(&self.inner) }
    }
}

impl Display for CanonicalPathBuf {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::CanonicalPath;
    use super::CanonicalPathBuf;

    #[test]
    fn insensitive_path_from_str() {
        assert_eq!(
            CanonicalPath::from_str("abc/def/123/ghi").unwrap().as_str(),
            "abc/def/123/ghi",
        );
        assert_eq!(CanonicalPath::from_str("abc/def/123\\ghi"), None);
        assert_eq!(CanonicalPath::from_str("abc/def/123/Ghi"), None);
    }

    #[test]
    fn insensitive_path_buf_from_string() {
        assert_eq!(
            CanonicalPathBuf::from_string("abc/def/123/ghi".to_string()),
            Some(CanonicalPathBuf {
                inner: "abc/def/123/ghi".to_string()
            }),
        );
        assert_eq!(
            CanonicalPathBuf::from_string("abc/def/123\\ghi".to_string()),
            None,
        );
        assert_eq!(
            CanonicalPathBuf::from_string("abc/def/123/Ghi".to_string()),
            None,
        );
    }
}
