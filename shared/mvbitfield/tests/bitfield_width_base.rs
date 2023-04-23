use mvbitfield::prelude::*;

bitfield! {
    struct BinaryStructWidth: 2 {
        x: 0b10,
    }

    struct OctalWidth: 8 {
        x: 0o10,
    }

    struct HexWidth: 16 {
        x: 0x10,
    }
}
