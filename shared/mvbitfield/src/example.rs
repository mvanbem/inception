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
    /// #     struct UserDefinedBitintBitfield: 3 { .. }
    /// #     struct UserDefinedPrimitiveBitfield: 8 { .. }
    /// # }
    /// bitfield! {
    ///     #[lsb_first]
    ///     pub struct ExampleA: 32 {
    ///         pub bit: 1,
    ///         pub flag: 1 as bool,
    ///         pub multi_bit: 5,
    ///         pub user_defined_bitint: 3 as UserDefinedBitintBitfield,
    ///         pub user_defined_primitive: 8 as UserDefinedPrimitiveBitfield,
    ///         ..
    ///     }
    /// }
    #[lsb_first]
    pub struct ExampleA: 32 {
        pub bit: 1,
        pub flag: 1 as bool,
        pub multi_bit: 5,
        pub user_defined_bitint: 3 as UserDefinedBitintBitfield,
        pub user_defined_primitive: 8 as UserDefinedPrimitiveBitfield,
    ..
    }
}

/// A custom bitfield type that wraps a [`u8`] primitive integer.
pub struct UserDefinedPrimitiveBitfield(pub u8);

impl From<U8> for UserDefinedPrimitiveBitfield {
    fn from(value: U8) -> Self {
        Self::from_bitint(value)
    }
}

impl From<UserDefinedPrimitiveBitfield> for U8 {
    fn from(value: UserDefinedPrimitiveBitfield) -> Self {
        value.to_bitint()
    }
}

impl Bitfield for UserDefinedPrimitiveBitfield {
    type Bitint = U8;

    const ZERO: Self = Self(0);

    fn zero() -> Self {
        Self::ZERO
    }

    fn from_bitint(value: U8) -> Self {
        Self(value.to_primitive())
    }

    fn to_bitint(self) -> U8 {
        U8::from_primitive(self.0)
    }
}

/// A custom bitfield type that wraps a [`U3`] `bitint`.
pub struct UserDefinedBitintBitfield(pub U3);

impl From<U3> for UserDefinedBitintBitfield {
    fn from(value: U3) -> Self {
        Self::from_bitint(value)
    }
}

impl From<UserDefinedBitintBitfield> for U3 {
    fn from(value: UserDefinedBitintBitfield) -> Self {
        value.to_bitint()
    }
}

impl Bitfield for UserDefinedBitintBitfield {
    type Bitint = U3;

    const ZERO: Self = Self(U3::ZERO);

    fn from_bitint(value: U3) -> Self {
        Self(value)
    }

    fn to_bitint(self) -> U3 {
        self.0
    }
}
