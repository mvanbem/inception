//! Narrow integer types.
//!
//! Each narrow integer type wraps a primitive integer type and imposes a validity constraint. The
//! value is represented in the least significant bits and the upper bits are always clear.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![no_std]

use core::fmt::{self, Display, Formatter};

pub mod prelude;
mod types;

#[doc(hidden)]
pub mod __private {
    pub use narrow_integer_macros::lit;
}

pub use types::*;

/// The error type returned when a checked narrow integer constructor fails.
#[derive(Debug)]
pub struct RangeError(pub(crate) ());

impl Display for RangeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "value out of range for narrow integer type")
    }
}

/// A specialized [`Result`] type for narrow integers.
pub type Result<T> = core::result::Result<T, RangeError>;

/// Narrow integer types, which wrap a primitive type and ensure the unused most significant bits
/// are clear.
pub trait NarrowInteger: Sized {
    /// The primitive type that this type wraps.
    type Primitive;

    /// The bit width of this type.
    const BITS: usize;
    /// The bit mask for the bits that may be set in values of this type.
    const MASK: Self::Primitive;

    /// The smallest value of this type.
    const MIN: Self;
    /// The largest value of this type.
    const MAX: Self;

    /// The value `0` represented in this type.
    const ZERO: Self;
    /// The value `1` represented in this type.
    const ONE: Self;

    /// Creates a narrow integer from a primitive value if it is in range for this type, as
    /// determined by [`is_in_range`](Self::is_in_range).
    fn new(value: Self::Primitive) -> Option<Self>;

    /// Creates a narrow integer by masking off the upper bits of a primitive value.
    ///
    /// This conversion is lossless if the value is in range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    fn new_masked(value: Self::Primitive) -> Self;

    /// Creates a narrow integer from a primitive value without checking whether it is in range for
    /// this type.
    ///
    /// # Safety
    ///
    /// The value must be in range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    unsafe fn new_unchecked(value: Self::Primitive) -> Self;

    /// Converts the value to a primitive type.
    ///
    /// The result is in range for this type, as determined by [`is_in_range`](Self::is_in_range).
    fn as_primitive(self) -> Self::Primitive;

    /// Checks whether a primitive value is in range for this type.
    ///
    /// There are a few equivalent ways to express this check.
    ///
    /// - The unused most significant bits are clear: `(value & !Self::MASK) == 0`
    /// - The value is between [`MIN`](Self::MIN) and [`MAX`](Self::MAX), inclusive: `value >=
    ///   Self::MIN.as_primitive() && value <= Self::MAX.as_primitive()`
    ///
    fn is_in_range(value: Self::Primitive) -> bool;

    /// The smallest value of this type.
    fn min() -> Self;

    /// The largest value of this type.
    fn max() -> Self;

    /// The value `0` represented in this type.
    fn zero() -> Self;

    /// The value `1` represented in this type.
    fn one() -> Self;
}

/// Constructs a narrow integer literal.
///
/// This macro accepts an integer literal with a custom suffix. The suffix must be the character
/// `'u'` followed by an integer, which must be the width of a narrow integer type in this crate.
/// The literal value is checked against the narrow integer type's range and is replaced with either
/// a call to a non-panicking const narrow integer constructor or a compile error.
///
/// # Examples
///
/// ```
/// # use narrow_integer::prelude::*;
/// // The suffix `u3` refers to the narrow integer type `U3`.
/// let x = lit!(6u3);
/// assert_eq!(x.as_u8(), 6);
///```
///
/// ```compile_fail
/// # use narrow_integer::prelude::*;
/// // This value is out of range for `U9`.
/// lit!(512u9);
/// ```
#[macro_export]
macro_rules! lit {
    ($lit:literal) => {
        $crate::__private::lit! { ($crate, $lit) }
    };
}

/// Rewrites narrow integer literals in the item it is attached to.
///
/// A narrow integer literal is an integer literal with a suffix consisting of `'u'` followed by an
/// integer, which must be the width of a narrow integer type in this crate.
///
/// Narrow integer literals are range checked and are replaced with either a call to a non-panicking
/// const narrow integer constructor or a compile error. All other tokens are preserved.
///
/// # Examples
///
/// ```
/// # use narrow_integer::prelude::*;
/// #[narrow_integer_literals]
/// fn example() {
///     let x = 6u3;
///     assert_eq!(x.as_u8(), 6);
/// }
/// ```
///
/// ```compile_fail
/// # use narrow_integer::prelude::*;
/// #[narrow_integer_literals]
/// fn example() {
///     // This value is out of range for `U9`.
///     let x = 512u9;
/// }
/// ```
pub use narrow_integer_macros::narrow_integer_literals;
