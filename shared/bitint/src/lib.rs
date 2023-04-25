#![cfg_attr(not(test), no_std)]
#![cfg_attr(feature = "unchecked_math", feature(unchecked_math))]
#![cfg_attr(feature = "_nightly", feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use core::fmt::{self, Display, Formatter};
use core::num::ParseIntError;

pub mod prelude;
mod traits;
pub mod types;

// For macro access via `$crate`.
#[doc(hidden)]
pub mod __private {
    pub use bitint_macros::bitint;
}

mod sealed {
    pub trait Sealed {}
}

/// The error type returned when a [`TryFrom`] conversion to a `bitint` fails.
#[derive(Debug)]
pub struct RangeError(pub(crate) ());

impl Display for RangeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "value out of range for bitint type")
    }
}

/// The error type returned when parsing a string to a `bitint` fails.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParseBitintError {
    /// Parsing failed because parsing to the primitive type failed.
    Parse(ParseIntError),
    /// Parsing failed because the primitive value was out of range.
    Range(RangeError),
}

impl Display for ParseBitintError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Range(e) => write!(f, "{e}"),
        }
    }
}

impl From<ParseIntError> for ParseBitintError {
    fn from(value: ParseIntError) -> Self {
        Self::Parse(value)
    }
}

impl From<RangeError> for ParseBitintError {
    fn from(value: RangeError) -> Self {
        Self::Range(value)
    }
}

pub use traits::*;

/// Constructs a `bitint` literal.
///
/// A `bitint` literal is an integer literal with a suffix consisting of `'U'`
/// followed by an integer, which must be at least one and at most 128.
///
/// This macro accepts one `bitint` literal which is checked against the
/// corresponding [`UBitint`] type's range and replaced with either a call to a
/// non-panicking const constructor or a compile error.
///
/// # Examples
///
/// ```
/// # use bitint::prelude::*;
/// // The suffix `U3` corresponds to the type `U3`. Underscores are permitted
/// // anywhere in a Rust literal and are encouraged for readability.
/// let x = bitint!(6_U3);
/// assert_eq!(x.to_primitive(), 6);
///```
///
/// ```compile_fail
/// # use bitint::prelude::*;
/// // This value is out of range for `U16`.
/// bitint!(65536_U16);
/// ```
#[macro_export]
macro_rules! bitint {
    ($($tt:tt)*) => {
        $crate::__private::bitint! { ($crate, $($tt)*) }
    };
}

/// Rewrites `bitint` literals in the item it is attached to.
///
/// A `bitint` literal is an integer literal with a suffix consisting of `'U'`
/// followed by an integer, which must be at least one and at most 128.
///
/// `bitint` literals are checked against the corresponding [`UBitint`] type's
/// range and replaced with either a call to a non-panicking const constructor
/// or a compile error. All other tokens are preserved.
///
/// # Examples
///
/// ```
/// # use bitint::prelude::*;
/// #[bitint_literals]
/// fn example() {
///     // The suffix `U3` corresponds to the type `U3`. Underscores are
///     // permitted anywhere in a Rust literal and are encouraged for
///     // readability.
///     let x = 6_U3;
///     assert_eq!(x.to_primitive(), 6);
/// }
/// ```
///
/// ```compile_fail
/// # use bitint::prelude::*;
/// #[bitint_literals]
/// fn example() {
///     // This value is out of range for `U16`.
///     let x = 65536_U16;
/// }
/// ```
pub use bitint_macros::bitint_literals;

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_debug() {
        assert_eq!(format!("{:?}", U1::new(1).unwrap()), "U1(1)");
        assert_eq!(format!("{:?}", U12::new(1234).unwrap()), "U12(1234)");
        assert_eq!(format!("{:?}", U16::new(65535).unwrap()), "U16(65535)");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", U1::new(1).unwrap()), "1");
        assert_eq!(format!("{}", U12::new(1234).unwrap()), "1234");
        assert_eq!(format!("{}", U16::new(65535).unwrap()), "65535");
    }

    #[test]
    fn trybuild_tests() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests_error/*.rs");
    }
}
