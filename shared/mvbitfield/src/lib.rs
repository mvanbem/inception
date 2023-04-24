#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![cfg_attr(feature = "_nightly", feature(doc_cfg))]
#![no_std]

use bitint::prelude::*;
use paste::paste;
use seq_macro::seq;

pub use bitint;

#[cfg(any(doc, feature = "doc"))]
#[cfg_attr(feature = "_nightly", doc(cfg(doc)))]
pub mod example;
#[cfg(any(doc, feature = "doc"))]
#[cfg_attr(feature = "_nightly", doc(cfg(doc)))]
pub mod overview;
pub mod prelude;

#[doc(hidden)]
pub mod __private {
    pub use mvbitfield_macros::bitfield;
}

mod sealed {
    pub trait Sealed {}
}

/// Bitfield struct types.
///
/// Bitfields have a `bitint` type and a primitive type. The `bitint` type represents the canonical
/// integer representation of this type.
///
/// There are zero-cost conversions between the `Self` and the `bitint` type, and from `Self` to the
/// primitive type. There is a zero-cost conversion from the primitive type to `Self` only if the
/// `bitint` type is [`PrimitiveSizedBitint`]. Checked conversions from the primitive type to
/// `Self` are always available.
pub trait Bitfield: From<Self::Bitint> {
    /// The [`UBitint`] type with zero-cost conversions to and from [`Self`].
    type Bitint: UBitint + From<Self>;

    /// The type's zero value.
    const ZERO: Self;

    /// Returns the type's zero value.
    fn zero() -> Self {
        Self::ZERO
    }

    /// Creates a bitfield value from a primitive value if it is in range for the `bitint` type.
    ///
    /// This is a convenience alias for [`UBitint::new`] and [`Bitfield::from_bitint`].
    fn new(value: <Self::Bitint as UBitint>::Primitive) -> Option<Self> {
        UBitint::new(value).map(Self::from_bitint)
    }

    /// Creates a bitfield value by masking off the upper bits of a primitive value.
    ///
    /// This is a convenience alias for [`UBitint::new_masked`] and [`Bitfield::from_bitint`].
    fn new_masked(value: <Self::Bitint as UBitint>::Primitive) -> Self {
        Self::from_bitint(UBitint::new_masked(value))
    }

    /// Creates a bitfield value from a primitive value without checking whether it is in range for
    /// the `bitint` type.
    ///
    /// This zero-cost conversion is a convenience alias for [`UBitint::new_unchecked`] and
    /// [`Bitfield::from_bitint`].
    ///
    /// # Safety
    ///
    /// The value must be in range for the `bitint` type, as determined by [`UBitint::is_in_range`].
    unsafe fn new_unchecked(value: <Self::Bitint as UBitint>::Primitive) -> Self {
        Self::from_bitint(UBitint::new_unchecked(value))
    }

    /// Creates a bitfield value from an `bitint` value.
    ///
    /// This is a zero-cost conversion.
    fn from_bitint(value: Self::Bitint) -> Self;

    /// Creates a bitfield value from a primitive value.
    ///
    /// This zero-cost conversion is a convenience alias for
    /// [`PrimitiveSizedBitint::from_primitive`] and [`Bitfield::from_bitint`].
    fn from_primitive(value: <Self::Bitint as UBitint>::Primitive) -> Self
    where
        Self::Bitint: PrimitiveSizedBitint,
    {
        Self::from_bitint(PrimitiveSizedBitint::from_primitive(value))
    }

    /// Converts the value to the `bitint` type.
    ///
    /// This is a zero-cost conversion.
    fn to_bitint(self) -> Self::Bitint;

    /// Converts the value to the primitive type.
    ///
    /// The result is in range for the bitint type, as determined by [`UBitint::is_in_range`].
    ///
    /// This zero-cost conversion is a convenience alias for [`UBitint::to_primitive`] and
    /// [`Bitfield::to_bitint`].
    fn to_primitive(self) -> <Self::Bitint as UBitint>::Primitive {
        self.to_bitint().to_primitive()
    }
}

impl Bitfield for bool {
    type Bitint = U1;

    const ZERO: Self = false;

    fn from_bitint(value: U1) -> Self {
        value.into()
    }

    fn to_bitint(self) -> U1 {
        self.into()
    }
}

/// Bitfield accessors.
///
/// This trait is implemented by all sized unsigned primitive integer types, all unsigned `bitint`s,
/// and any [`Bitfield`].
///
/// Provides methods used in generated bitfield structs. Not intended to be brought into scope
/// because [`to_primitive`](Self::to_primitive) is ambiguous with [`Bitfield::to_primitive`].
pub trait Accessor: crate::sealed::Sealed {
    /// The primitive type that this type wraps.
    type Primitive;

    /// Creates an accessor value by masking off the upper bits of a primitive value.
    fn from_primitive_masked(value: Self::Primitive) -> Self;

    /// Creates an accessor value from a primitive value without checking whether it is in range for
    /// the `bitint` type.
    ///
    /// This is a zero-cost conversion.
    ///
    /// # Safety
    ///
    /// * For sized unsigned primitive integer types, always safe.
    /// * For unsigned `bitint`s, the value must be in range, as determined by
    ///   [`UBitint::is_in_range`].
    /// * For [`Bitfield`]s, the value must be in range for the `bitint` type, as determined by
    ///   [`UBitint::is_in_range`].
    unsafe fn from_primitive_unchecked(value: Self::Primitive) -> Self;

    /// Converts the value to the primitive type.
    ///
    /// This is a zero-cost conversion.
    fn to_primitive(self) -> Self::Primitive;
}

macro_rules! impl_accessor {
    (primitives: $($ty:ty),*) => {
        paste! {
            $(
                impl crate::sealed::Sealed for $ty {}

                impl Accessor for $ty {
                    type Primitive = Self;

                    fn from_primitive_masked(value: Self) -> Self {
                        value
                    }

                    unsafe fn from_primitive_unchecked(value: Self) -> Self {
                        value
                    }

                    fn to_primitive(self) -> Self {
                        self
                    }
                }
            )*
        }
    };
    (ubitint: $width:literal) => {
        paste! {
            impl crate::sealed::Sealed for [<U $width>] {}

            impl Accessor for [<U $width>] {
                type Primitive = <Self as UBitint>::Primitive;

                fn from_primitive_masked(value: Self::Primitive) -> Self {
                    Self::new_masked(value)
                }

                unsafe fn from_primitive_unchecked(value: Self::Primitive) -> Self {
                    Self::new_unchecked(value)
                }

                fn to_primitive(self) -> Self::Primitive {
                    self.to_primitive()
                }
            }
        }
    };
}
impl_accessor!(primitives: u8, u16, u32, u64, u128);
seq!(N in 1..=128 { impl_accessor!(ubitint: N); });

impl<T: Bitfield> crate::sealed::Sealed for T {}

impl<T: Bitfield> Accessor for T {
    type Primitive = <T::Bitint as UBitint>::Primitive;

    fn from_primitive_masked(value: Self::Primitive) -> Self {
        Self::new_masked(value)
    }

    unsafe fn from_primitive_unchecked(value: Self::Primitive) -> Self {
        Self::new_unchecked(value)
    }

    fn to_primitive(self) -> Self::Primitive {
        self.to_primitive()
    }
}

/// Generates a bitfield struct.
///
/// See the [overview of concepts and terms](overview) and [examples](example).
///
#[doc = include_str!("../syntax.md")]
#[macro_export]
macro_rules! bitfield {
    ($($tt:tt)*) => {
        $crate::__private::bitfield! { ($crate, $($tt)*) }
    };
}

#[test]
fn trybuild_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests_error/*.rs");
}
