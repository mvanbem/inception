Yet another bitfield crate.

`mvbitfield` generates bitfield structs that wrap integer types and can insert and extract
bitfields.

The generated bitfield structs are:

- **Endian-insensitive**, only packing bitfields within an integer, never across array elements.
- **Suitable for FFI and memory-mapped I/O**, having the same layout as a primitive integer type.
- **Type-safe**, using [`bitint`](bitint)s to model bitfield widths and guarantee unused upper bits
  are clear.
- **Flexible**, with support for user-defined bitfield accessor types.

# Demo

```
// Recommended, but not required. The mvbitfield prelude includes the bitint prelude.
use mvbitfield::prelude::*;

bitfield! {
    #[lsb_first]                      // Field packing order
    #[derive(PartialEq, Eq)]          // Other attributes are passed through
    pub struct MyBitfieldStruct: 8 {  // Eight bits wide
        _padding: 1,                  // No accessors when name starts with _
        pub some_number: 3,           // Public U3 accessors
        ..,                           // Skip unused bits
        high_bit_flag: 1 as bool,     // Private bool accessors
    }
}

// Use generated with_* methods to build bitfield structs.
let x = MyBitfieldStruct::zero()
    .with_some_number(lit!(6u3))
    .with_high_bit_flag(true);

// Default accessors always return bitints.
assert_eq!(x.some_number(), lit!(6u3));
assert_eq!(x.some_number().to_primitive(), 6u8);

// Custom accessors always return types that impl Bitfield.
assert_eq!(x.high_bit_flag(), true);
assert_eq!(x.high_bit_flag().to_bitint(), lit!(1u1));
assert_eq!(x.high_bit_flag().to_primitive(), 1u8);

// Zero-cost conversions involving bitfield structs.
assert_eq!(x.to_bitint(), lit!(0b1_000_110_0u8));
assert_eq!(x.to_primitive(), 0b1_000_110_0u8);
assert_eq!(x, MyBitfieldStruct::from_bitint(lit!(0b1_000_110_0u8)));

// Zero-cost conversion from primitive to primitive-sized bitfield struct.
assert_eq!(x, MyBitfieldStruct::from_primitive(0b1_000_110_0u8));
```

# Getting Started

* Read the [overview] and take a look at some [examples](example).
* Consult the syntax reference under the [bitfield!] macro.
