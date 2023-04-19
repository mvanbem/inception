use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    struct FullyOutsideMsbFirst: u8 {
        x: 8,
        y: 1,
    }

    #[lsb_first]
    struct FullyOutsideLsbFirst: u8 {
        x: 8,
        y: 1,
    }

    #[msb_first]
    struct SplitMsbFirst: u8 {
        x: 7,
        y: 2,
    }

    #[lsb_first]
    struct SplitLsbFirst: u8 {
        x: 7,
        y: 2,
    }
}

fn main() {}
