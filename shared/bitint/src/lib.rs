//! Integer types that have a logical size measured in bits.
//!
//! This crate provides the [`BitUint`] trait and 128 types named [`U1`](crate::types::U1) through
//! [`U128`](crate::types::U128) that implement it. Each type wraps the smallest primitive unsigned
//! integer type that can contain it. The types that are not the same width as a primitive unsigned
//! integer type impose a validity constraint---the value is represented in the least significant
//! bits and the upper bits are always clear.
//!
//! # Features

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![no_std]

use core::fmt::{self, Display, Formatter};

pub mod prelude;
pub mod types;

#[doc(hidden)]
pub mod __private {
    pub use bitint_macros::lit;
}

mod sealed {
    pub trait Sealed {}
}

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

/// Unsigned integer types that have a logical width measured in bits.
///
/// There is one type implementing `BitUint` for each bit width from 1 to 128 inclusive.
pub trait BitUint: Sized + sealed::Sealed {
    /// The primitive type that this type wraps.
    type Primitive: From<Self>;

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

    /// Creates a bit-sized value from a primitive value if it is in range for this type, as
    /// determined by [`is_in_range`](Self::is_in_range).
    fn new(value: Self::Primitive) -> Option<Self>;

    /// Creates a bit-sized value by masking off the upper bits of a primitive value.
    ///
    /// This conversion is lossless if the value is in range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    fn new_masked(value: Self::Primitive) -> Self;

    /// Creates a bit-sized value from a primitive value without checking whether it is in range for
    /// this type.
    ///
    /// This is a zero-cost conversion.
    ///
    /// # Safety
    ///
    /// The value must be in range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    unsafe fn new_unchecked(value: Self::Primitive) -> Self;

    /// Converts the value to a primitive type.
    ///
    /// This is a zero-cost conversion. The result is in range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    fn to_primitive(self) -> Self::Primitive;

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

/// Unsigned integer types that are the same width as a primitive integer type.
pub trait PrimitiveSizedBitUint: BitUint + From<Self::Primitive> {
    /// Creates a bit-sized value from a primitive value of the same width.
    ///
    /// This is a zero-cost conversion.
    fn from_primitive(value: Self::Primitive) -> Self;
}

/// Constructs a `bitint` literal.
///
/// A `bitint` literal is an integer literal with a suffix consisting of `'u'` followed by an
/// integer, which must be at least one and at most 128.
///
/// This macro accepts one `bitint` literal which is checked against the corresponding [`BitUint`]
/// type's range and replaced with either a call to a non-panicking const constructor or a compile
/// error.
///
/// # Examples
///
/// ```
/// # use bitint::prelude::*;
/// // The suffix `u3` corresponds to the type `U3`.
/// let x = lit!(6u3);
/// assert_eq!(x.to_primitive(), 6);
///```
///
/// ```compile_fail
/// # use bitint::prelude::*;
/// // This value is out of range for `U16`.
/// lit!(65536u16);
/// ```
#[macro_export]
macro_rules! lit {
    ($lit:literal) => {
        $crate::__private::lit! { ($crate, $lit) }
    };
}

/// Rewrites `bitint` literals in the item it is attached to.
///
/// A `bitint` literal is an integer literal with a suffix consisting of `'u'` followed by an
/// integer, which must be at least one and at most 128.
///
/// `bitint` literals are checked against the corresponding [`BitUint`] type's range and replaced
/// with either a call to a non-panicking const constructor or a compile error. All other tokens are
/// preserved.
///
/// # Examples
///
/// ```
/// # use bitint::prelude::*;
/// #[bitint_literals]
/// fn example() {
///     // The suffix `u3` corresponds to the type `U3`.
///     let x = 6u3;
///     assert_eq!(x.to_primitive(), 6);
/// }
/// ```
///
/// ```compile_fail
/// # use bitint::prelude::*;
/// #[bitint_literals]
/// fn example() {
///     // This value is out of range for `U16`.
///     let x = 65536u16;
/// }
/// ```
pub use bitint_macros::bitint_literals;
