use mvbitfield::prelude::*;

bitfield! {
    struct BinaryWidth: 0b10 {
        x: 2,
    }

    struct OctalWidth: 0o10 {
        x: 8,
    }

    struct HexWidth: 0x10 {
        x: 16,
    }
}
