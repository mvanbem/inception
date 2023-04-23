use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    struct FullyOutsideMsbFirst: 8 {
        x: 8,
        y: 1,
    }

    #[lsb_first]
    struct FullyOutsideLsbFirst: 8 {
        x: 8,
        y: 1,
    }

    #[msb_first]
    struct SplitMsbFirst: 8 {
        x: 7,
        y: 2,
    }

    #[lsb_first]
    struct SplitLsbFirst: 8 {
        x: 7,
        y: 2,
    }
}

fn main() {}
