//! An overview of `mvbitfield` concepts and terms.
//!
//! # Bitfield Structs
//!
//! The [`bitfield!`](crate::bitfield) macro generates bitfield structs.
//! Bitfield structs are declared with a sequence of fields, but unlike regular
//! Rust structs those fields are not directly exposed. Instead, they are stored
//! in statically packed ranges of bits in an underlying integer type, and are
//! accessible only by value through methods that perform the necessary shifting
//! and masking operations.
//!
//! # Primitive and Narrow Integer Types
//!
//! The Rust Core Library provides five sized primitive unsigned integer types:
//! [`u8`], [`u16`], [`u32`], [`u64`], and [`u128`]. `mvbitfield` augments this
//! set with 123 [narrow integer types](bitint) named [`U1`](crate::prelude::U1)
//! through [`U127`](crate::prelude::U127) to fill in the gaps.
//!
//! # Underlying Type
//!
//! Each bitfield struct specifies an underlying type, which may be either a
//! primitive unsigned integer type or a [narrow integer type](bitint).
//!
//! The generated struct has the same layout as the underlying type and can be
//! [converted to and from that type](#underlying-type-conversion) for free in a
//! const context.
//!
//! Generated structs with primitive integer underlying types are particularly
//! well suited for memory-mapped I/O and foreign function interface bindings
//! because they have no forbidden bit patterns.
//!
//! # Bitfield Packing
//!
//! Bitfields occupy contiguous ranges of bits in the underlying type and are
//! tightly packed in declaration order. Every underlying bit is covered by one
//! bitfield. The `..` shorthand for a flexible reserved bitfield may be
//! convenient to ensure every bit is covered.
//!
//! Packing begins with the first declared bitfield at either the least or most
//! significant bit. The packing direction is determined by the mandatory
//! [`#[lsb_first]` or `#[msb_first]` struct
//! attribute](crate::bitfield#struct-attributes).
//!
//! # Bitfield Accessor Types
//!
//! Each bitfield's default accessor type is the unique primitive integer type
//! or [narrow integer type](bitint) with the same width as the bitfield. A
//! bitfield's accessor type may be overridden with the optional `as` syntax in
//! the [bitfield declaration](crate::bitfield#bitfields).
//!
//! As a special case, bitfields with width 1 may declare [`bool`] accessors.
//!
//! Bitfields of any width may declare a user-defined accessor type, which must
//! provide the following methods:
//!
//! ```ignore
//! const fn from_uN(value: T) -> Self;
//! const fn as_uN(self) -> T;
//! ```
//!
//! where `T` is the bitfield's default accessor type and `N` is its width.
//!
//! All generated bitfield struct types [provide these
//! methods](#underlying-type-conversion) and are valid accessor types for
//! bitfields of the same width.
//!
//! # Bitfield Struct API
//!
//! ## Trait Implementations
//!
//! All generated types invoke the [`Clone`], [`Copy`], [`Debug`],
//! [`PartialEq`], [`Eq`], and [`Hash`] derive macros.
//!
//! ## Constructors and Conversions
//!
//! ### Zero Constructor
//!
//! All generated types provide this method:
//!
//! ```ignore
//! pub const fn zero() -> Self;
//! ```
//!
//! ### Underlying Type Conversion
//!
//! All generated types provide these methods:
//!
//! ```ignore
//! pub const fn from_uN(value: T) -> Self;
//! pub const fn as_uN(self) -> T;
//! ```
//!
//! where `T` is the underlying type and `N` is its width.
//!
//! ### Nested Primitive Type Conversion
//!
//! Generated types with narrow integer underlying types also provide these
//! convenience methods for the nested primitive type:
//!
//! ```ignore
//! pub const fn new(value: T) -> Option<Self>;
//! pub const fn new_masked(value: T) -> Self;
//! pub const unsafe fn new_unchecked(value: T) -> Self;
//! pub const fn as_uN(self) -> T;
//! ```
//!
//! where `T` is the nested primitive type and `N` is its width.
//!
//! ## Bitfield Accessors
//!
//! Any methods generated for a bitfield use the visibility specifier from the
//! [bitfield declaration](crate::bitfield#bitfields).
//!
//! A bitfield is _reserved_ if its name starts with an underscore; otherwise it
//! is _defined_. Reserved bitfields do not generate accessor methods.
//!
//! All defined bitfields provide a getter, two functional setters, and two
//! mutating setters:
//!
//! ```ignore
//! const fn        B(self                             ) -> T;
//! const fn   with_B(self, value: T                   ) -> Self;
//!       fn    map_B(self, f: impl FnOnce(T) -> T     ) -> Self;
//! const fn    set_B(&mut self, value: T              );
//!       fn modify_B(&mut self, f: impl FnOnce(T) -> T);
//! ```
//!
//! where `B` is the bitfield's name and `T` is the bitfield's accessor type.
//!
//! ### Nested Primitive Accessors
//!
//! Bitfields with narrow integer accessor types also provide convenience
//! methods for the nested primitive type.
//!
//! The getter converts the accessor type to the nested primitive type:
//!
//! ```ignore
//! const fn B_uN(self) -> T;
//! ```
//!
//! where `B` is the bitfield's name, `N` is the nested primitive type's width,
//! and `T` is the nested primitive type.
//!
//! The functional setters have variants that compose each of the narrow integer
//! type constructors with the operation:
//!
//! ```ignore
//! const        fn with_B_uN          (self, value: T) -> Option<Self>;
//! const        fn with_B_uN_masked   (self, value: T) -> Self;
//! const unsafe fn with_B_uN_unchecked(self, value: T) -> Self;
//!
//!        fn map_B_uN          (self, f: impl FnOnce(T) -> T) -> Option<Self>;
//!        fn map_B_uN_masked   (self, f: impl FnOnce(T) -> T) -> Self;
//! unsafe fn map_B_uN_unchecked(self, f: impl FnOnce(T) -> T) -> Self;
//! ```
//!
//! where `B` is the bitfield's name, `N` is the nested primitive type's width,
//! and `T` is the nested primitive type.
//!
//! The mutating setters have similar variants, but with the fallible methods
//! returning a `Result<Self, bitint::RangeError>`]`.
//!
//! ```ignore
//! const        fn set_B_uN          (&mut self, value: T) -> Result<(), RangeError>;
//! const        fn set_B_uN_masked   (&mut self, value: T);
//! const unsafe fn set_B_uN_unchecked(&mut self, value: T);
//!
//!        fn modify_B_uN          (&mut self, f: impl FnOnce(T) -> T) -> Result<(), RangeError>;
//!        fn modify_B_uN_masked   (&mut self, f: impl FnOnce(T) -> T);
//! unsafe fn modify_B_uN_unchecked(&mut self, f: impl FnOnce(T) -> T);
//! ```
//!
//! where `B` is the bitfield's name, `N` is the nested primitive type's width,
//! and `T` is the nested primitive type.
//!
