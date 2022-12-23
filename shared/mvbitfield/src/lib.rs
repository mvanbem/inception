//! Yet another bitfield crate.
//!
//! `mvbitfield` provides a proc macro and helper types for inserting and extracting bitfields.
//!
//! The generated types are:
//!
//! - **Endian insensitive**, only packing bitfields within an integer, never across array elements.
//! - **Suitable for FFI and memory-mapped I/O**, having the same layout as the underlying primitive
//!   integer type.
//! - **Const-friendly**, with bitfield insertion and extraction methods available in a const
//!   context.
//! - **Clear**, using narrow integer types to model field widths and guarantee the states of upper
//!   bits.
//! - **Flexible**, allowing user-defined field types so you can color your integers.
//!
//! I found these properties convenient for developing a toy embedded operating system.

#![no_std]

use core::fmt::{self, Display, Formatter};

pub mod narrow_integer;
pub mod prelude;

/// Generates a type that wraps an integer, providing methods to insert and extract bitfields.
///
/// The generated type has the same layout as the underlying integer type.
///
/// # Example
///
/// ```
/// mvbitfield! {
///     //                  +------- Name of the generated type
///     //                  |    +-- Underlying type
///     //         |--------|  |-|
///     pub struct MyBitfield: u32 {
///
///         // Bitfields are packed starting from the LSB. Multi-bit bitfields have their MSBs and
///         // LSBs oriented the same way as the underlying primitive type.
///
///         //    +----- Field name
///         //    |  +-- Field bit width
///         //    |  |   => pub const fn foo(self) -> U6
///         //    |  |   => pub const fn with_foo(self, value: U6) -> Self
///         //  |-|  |   => pub const fn set_foo(&mut self, value: U6)
///         pub foo: 6,
///
///         // A field name starting with an underscore generates no methods.
///         _skip: 2,
///
///         //                +-- Field type override
///         //                |   => pub const fn flag(self) -> bool
///         //                |   => pub const fn with_flag(self, value: bool) -> Self
///         //          |-----|   => pub const fn set_flag(&mut self, value: bool)
///         pub flag: 1 as bool,
///
///         //                     +-- User-defined field type
///         //                     |   => pub const fn my_type(self) -> MyType
///         //                     |   => pub const fn with_my_type(self, value: MyType) -> Self
///         //             |-------|   => pub const fn set_my_type(&mut self, value: MyType)
///         pub my_type: 3 as MyType,
///     }
/// }
/// ```
///
/// # DSL Spec
///
/// ```
/// mvbitfield! { <struct_decl> }
///
/// <struct_decl>     := <struct_header> <struct_body>;
/// <struct_header>   := <visibility_spec> "struct" $ident ":" $ident;
/// <struct_body>     := "{" <field_spec> ("," <field_spec>)* "}";
/// <field_spec>      := <visibility_spec> $ident ":" $literal ("as" $ident)?;
/// <visibility_spec> := empty
///                    | "pub"
///                    | "pub" "(" "crate" ")";
/// ```
///
/// # Underlying Type
///
/// The underlying type of a bitfield struct may be either a primitive integer type or a
/// [narrow integer type](crate::narrow_integer).
///
/// Bitfield structs with primitive integer underlying types are intended for MMIO access and FFI.
/// They have no forbidden bit patterns and can be safely constructed from a primitive integer for
/// free.
///
/// Bitfield structs with narrow integer underlying types are intended for use as user-defined field
/// types within other bitfield structs. They have forbidden bit patterns and cannot be safely
/// constructed for free from a primitive integer.
///
/// # Field Types
///
/// The default field type is the unique primitive integer type or
/// [narrow integer type](crate::narrow_integer) with the same bit width as the field. So a 16-bit
/// field is inserted and extracted as a `u16`, while an 18-bit field is inserted and extracted as a
/// [`U18`](crate::narrow_integer::U18).
///
/// ## Field Type Override
///
/// One-bit fields may declare `bool` accessors instead of the default `U1` with the `as` keyword.
///
/// ## User-Defined Field Types
///
/// Fields of any width may declare a user-defined type for accessors with the `as` keyword. All
/// generated bitfield structs are valid user-defined field types.
///
/// User-defined field types must have the following methods, where `N` is a placeholder for the
/// field bit width and `T` is the field's default type:
///
/// ```
/// const fn as_uN(self) -> T;
/// const fn from_uN(value: T) -> Self;
/// ```
///
/// For a seven-bit user-defined type, those would be:
///
/// ```
/// const fn as_u7(self) -> U7;
/// const fn from_u7(value: U7) -> Self;
/// ```
///
/// And for a 32-bit user-defined type:
///
/// ```
/// const fn as_u32(self) -> u32;
/// const fn from_u32(value: u32) -> Self;
/// ```
pub use mvbitfield_proc_macro::mvbitfield;

#[derive(Debug)]
pub struct InvalidBitPattern;

impl Display for InvalidBitPattern {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "invalid bit pattern")
    }
}
