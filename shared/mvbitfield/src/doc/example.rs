//! Example invocations, generated types, and usage.

use crate::prelude::*;

bitfield! {
    /// A bitfield struct wrapping a `u32`.
    ///
    /// # Declaration
    ///
    /// ```
    /// # use mvbitfield::prelude::*;
    /// # bitfield! {
    /// #     struct PrimitiveCustomField: 8 { .. }
    /// # }
    /// bitfield! {
    ///     #[lsb_first]
    ///     pub struct ExampleA: 32 {
    ///         pub bit: 1,
    ///         pub flag: 1 as bool,
    ///         pub narrow_field: 5,
    ///         pub primitive_field: 8,
    ///         pub primitive_custom_field: 8 as PrimitiveCustomField,
    ///         ..
    ///     }
    /// }
    #[lsb_first]
    pub struct ExampleA: 32 {
        pub bit: 1,
        pub flag: 1 as bool,
        pub narrow_field: 5,
        pub primitive_field: 8,
        pub primitive_custom_field: 8 as PrimitiveCustomField,
        ..
    }
}

/// A custom field type that wraps a [`u8`] primitive integer.
pub struct PrimitiveCustomField(pub u8);

impl From<u8> for PrimitiveCustomField {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<PrimitiveCustomField> for u8 {
    fn from(value: PrimitiveCustomField) -> Self {
        value.0
    }
}

impl Bitfield for PrimitiveCustomField {
    type Underlying = u8;

    const ZERO: Self = Self(0);

    fn zero() -> Self {
        Self::ZERO
    }

    fn from_underlying(value: Self::Underlying) -> Self {
        Self(value)
    }

    fn to_underlying(self) -> Self::Underlying {
        self.0
    }
}

/// A custom field type that wraps a [`U3`] narrow integer.
pub struct NarrowCustomField(pub U3);

impl NarrowCustomField {
    /// Required method for use as an mvbitfield custom field type.
    pub const fn as_u3(self) -> U3 {
        self.0
    }

    /// Required method for use as an mvbitfield custom field type.
    pub const fn from_u3(value: U3) -> Self {
        Self(value)
    }
}