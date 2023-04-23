#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![feature(doc_cfg)]
#![no_std]

use bitint::prelude::*;
use paste::paste;
use seq_macro::seq;

pub use bitint;

// #[cfg(doc)]
pub mod doc;

pub mod prelude;

#[doc(hidden)]
pub mod __private {
    pub use mvbitfield_macros::bitfield;
}

/// Bitfield struct and accessor types.
///
/// Bitfields have an [`Underlying`](Self::Underlying) type implementing [`BitUint`] and provide
/// zero-cost conversions to and from that type. Bitfields also have a primitive type,
/// [`Underlying::Primitive`](BitUint::Primitive), and provide a zero-cost conversion to that type.
pub trait Bitfield: From<Self::Underlying> {
    /// The underlying type, freely convertible to and from [`Self`].
    type Underlying: BitUint + From<Self>;

    /// The type's zero value.
    const ZERO: Self;

    /// Returns the type's zero value.
    fn zero() -> Self {
        Self::ZERO
    }

    /// Creates a bitfield value from an underlying value.
    ///
    /// This is a zero-cost conversion.
    fn from_underlying(value: Self::Underlying) -> Self;

    /// Creates a bitfield value from a primitive value.
    ///
    /// This conversion is available only when the underlying and primitive types are the
    /// same, and is a convenience alias for the corresponding [`From`] implementation.
    fn from_primitive(value: <Self::Underlying as BitUint>::Primitive) -> Self
    where
        Self: From<<Self::Underlying as BitUint>::Primitive>,
    {
        value.into()
    }

    /// Creates a bitfield value from a primitive value if it is in range for the underlying type.
    ///
    /// This is a convenience alias for [`BitUint::new`] and [`Bitfield::from_underlying`].
    fn new(value: <Self::Underlying as BitUint>::Primitive) -> Option<Self> {
        <Self::Underlying as BitUint>::new(value).map(Self::from_underlying)
    }

    /// Creates a bitfield value by masking off the upper bits of a primitive value.
    ///
    /// This is a convenience alias for [`BitUint::new_masked`] and [`Bitfield::from_underlying`].
    fn new_masked(value: <Self::Underlying as BitUint>::Primitive) -> Self {
        Self::from_underlying(<Self::Underlying as BitUint>::new_masked(value))
    }

    /// Creates a bitfield value from a primitive value without checking whether it is in range for
    /// the underlying type.
    ///
    /// This zero-cost conversion is a convenience alias for [`BitUint::new_unchecked`] and
    /// [`Bitfield::from_underlying`].
    ///
    /// # Safety
    ///
    /// The value must be in range for the underlying type, as determined by
    /// [`BitUint::is_in_range`].
    unsafe fn new_unchecked(value: <Self::Underlying as BitUint>::Primitive) -> Self {
        Self::from_underlying(<Self::Underlying as BitUint>::new_unchecked(value))
    }

    /// Converts the value to the underlying type.
    ///
    /// This is a zero-cost conversion.
    fn to_underlying(self) -> Self::Underlying;

    /// Converts the value to the primitive type.
    ///
    /// The result is in range for the underlying type, as determined by [`BitUint::is_in_range`].
    ///
    /// This zero-cost conversion is a convenience alias for [`BitUint::to_primitive`] and
    /// [`Bitfield::to_underlying`].
    fn to_primitive(self) -> <Self::Underlying as BitUint>::Primitive {
        BitUint::to_primitive(self.to_underlying())
    }
}

impl Bitfield for bool {
    type Underlying = U1;

    const ZERO: Self = false;

    fn from_underlying(value: U1) -> Self {
        value.into()
    }

    fn to_underlying(self) -> U1 {
        self.into()
    }
}

macro_rules! impl_bitfield_for_primitives {
    ($($ty:ty),*) => {$(
        impl Bitfield for $ty {
            type Underlying = Self;

            const ZERO: Self = 0;

            fn from_underlying(value: Self) -> Self {
                value
            }

            fn to_underlying(self) -> Self {
                self
            }
        }
    )*};
}
impl_bitfield_for_primitives!(u8, u16, u32, u64, u128);

macro_rules! impl_bitfield_for_bitint {
    ($width:literal) => {
        paste! {
            impl Bitfield for [<U $width>] {
                type Underlying = Self;

                const ZERO: Self = <[<U $width>] as BitUint>::ZERO;

                fn from_underlying(value: Self) -> Self {
                    value
                }

                fn to_underlying(self) -> Self {
                    self
                }
            }
        }
    };
}
seq!(N in 1..8 { impl_bitfield_for_bitint!(N); });
seq!(N in 9..16 { impl_bitfield_for_bitint!(N); });
seq!(N in 17..32 { impl_bitfield_for_bitint!(N); });
seq!(N in 33..64 { impl_bitfield_for_bitint!(N); });
seq!(N in 65..128 { impl_bitfield_for_bitint!(N); });

/// Generates a bitfield struct.
///
/// See the [`doc`] module for an [overview of concepts and terms](doc::overview) and
/// [examples](doc::example).
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
