Yet another bitfield crate.

`mvbitfield` generates bitfield structs that wrap integer types and can insert and extract
bitfields.

The generated bitfield structs are:

- **Endian-insensitive**, only packing bitfields within an integer, never across array elements.
- **Suitable for FFI and memory-mapped I/O**, having the same layout as the underlying primitive
  integer type.
- **Const-friendly**, with bitfield insertion and extraction methods available in a const context.
- **Clear and efficient**, using [narrow integer types](narrow_integer) to model bitfield widths and
  guarantee unused upper bits are clear.
- **Flexible**, with support for user-defined bitfield accessor types.

# Demo

```
use mvbitfield::prelude::*;  // Not required, but nice
// Types like `U3` are in the prelude but also accessible as `mvbitfield::narrow_integer::U3`.

bitfield! {
    #[derive(PartialEq, Eq)]           // Passed through
    #[lsb_first]                       // Field packing order
    pub struct MyBitfieldStruct: u8 {  // Eight bits wide
        _padding: 1,                   // No accessors when name starts with _
        pub some_number: 3,            // Public U3 accessors
        ..,                            // Reserve any unused bits here
        high_bit_flag: 1 as bool,      // Private bool accessors
    }
}

let value: MyBitfieldStruct = MyBitfieldStruct::zero()
    .with_some_number(lit!(6u3))
    .with_high_bit_flag(true);

assert_eq!(value.some_number(), lit!(6u3));
assert_eq!(value.some_number().as_u8(), 6);
assert_eq!(value.as_u8(), 0b1_000_110_0);
assert_eq!(value, MyBitfieldStruct::from_u8(0b1_000_110_0));
```

# Getting Started

* Read the [overview](doc::overview) and take a look at some [examples](doc::example) in the [`doc`]
  module.
* Consult the syntax reference under the [bitfield!] macro.
