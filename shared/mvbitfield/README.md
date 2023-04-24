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
use mvbitfield::prelude::*;  // Not required, but nice
// The mvbitfield prelude includes the bitint prelude.

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

// bitint getters for fields with default accessors.
assert_eq!(x.some_number(), lit!(6u3));
// Use BitUint trait methods to manipulate these values.
assert_eq!(x.some_number().to_primitive(), 6u8);

// Custom getters for fields with accessor type overrides.
assert_eq!(x.high_bit_flag(), true);
// Use Bitfield trait methods to manipulate these values.
assert_eq!(x.high_bit_flag().to_underlying(), lit!(1u1));
assert_eq!(x.high_bit_flag().to_primitive(), 1u8);

// Zero-cost conversions between bitfield structs and bitints.
assert_eq!(x.to_underlying(), lit!(0b1_000_110_0u8));
assert_eq!(x, MyBitfieldStruct::from_underlying(lit!(0b1_000_110_0u8)));

// Zero-cost conversion from bitfield struct to primitive.
assert_eq!(x.to_primitive(), 0b1_000_110_0u8);

// Zero-cost conversion from primitive to primitive-sized bitfield struct.
assert_eq!(x, MyBitfieldStruct::from_primitive(0b1_000_110_0u8));
```

# Getting Started

* Read the [overview] and take a look at some [examples](example).
* Consult the syntax reference under the [bitfield!] macro.
