//! Integer types that have a logical size measured in bits.
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
    pub use bitint_macros::lit;
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

/// Unsigned integer types that have a logical width measured in bits.
pub trait BitUint: Sized {
    /// The primitive type that this type wraps. For primitive integers this is `Self`.
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

macro_rules! impl_bit_uint_for_primitive {
    ($ty:ty) => {
        impl BitUint for $ty {
            type Primitive = Self;

            const BITS: usize = Self::BITS as usize;
            const MASK: Self = Self::MAX;
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;
            const ZERO: Self = 0;
            const ONE: Self = 1;

            fn new(value: Self) -> Option<Self> {
                Some(value)
            }

            fn new_masked(value: Self) -> Self {
                value
            }

            unsafe fn new_unchecked(value: Self) -> Self {
                value
            }

            fn to_primitive(self) -> Self {
                self
            }

            fn is_in_range(_value: Self) -> bool {
                true
            }

            fn min() -> Self {
                Self::MIN
            }

            fn max() -> Self {
                Self::MAX
            }

            fn zero() -> Self {
                Self::ZERO
            }

            fn one() -> Self {
                Self::ONE
            }
        }
    };
}
impl_bit_uint_for_primitive!(u8);
impl_bit_uint_for_primitive!(u16);
impl_bit_uint_for_primitive!(u32);
impl_bit_uint_for_primitive!(u64);
impl_bit_uint_for_primitive!(u128);

impl BitUint for bool {
    type Primitive = u8;

    const BITS: usize = 1;
    const MASK: u8 = 1;
    const MIN: Self = false;
    const MAX: Self = true;
    const ZERO: Self = false;
    const ONE: Self = true;

    fn new(value: u8) -> Option<Self> {
        match value {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    fn new_masked(value: u8) -> Self {
        (value & 1) != 0
    }

    unsafe fn new_unchecked(value: u8) -> Self {
        // SAFETY: `bool` and `u8` have the same size (1) and alignment (1). The caller promised
        // that the value is 0 or 1.
        core::mem::transmute(value)
    }

    fn to_primitive(self) -> u8 {
        self as u8
    }

    fn is_in_range(value: u8) -> bool {
        value <= 1
    }

    fn min() -> Self {
        Self::MIN
    }

    fn max() -> Self {
        Self::MAX
    }

    fn zero() -> Self {
        Self::ZERO
    }

    fn one() -> Self {
        Self::ONE
    }
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
/// # use bitint::prelude::*;
/// // The suffix `u3` refers to the narrow integer type `U3`.
/// let x = lit!(6u3);
/// assert_eq!(x.to_primitive(), 6);
///```
///
/// ```compile_fail
/// # use bitint::prelude::*;
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
/// # use bitint::prelude::*;
/// #[bitint_literals]
/// fn example() {
///     let x = 6u3;
///     assert_eq!(x.to_primitive(), 6);
/// }
/// ```
///
/// ```compile_fail
/// # use bitint::prelude::*;
/// #[bitint_literals]
/// fn example() {
///     // This value is out of range for `U9`.
///     let x = 512u9;
/// }
/// ```
pub use bitint_macros::bitint_literals;
