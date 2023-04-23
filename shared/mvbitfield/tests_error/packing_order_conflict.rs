use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    #[msb_first]
    struct MsbFirstTwice: 8 {}

    #[lsb_first]
    #[lsb_first]
    struct LsbFirstTwice: 8 {}

    #[lsb_first]
    #[msb_first]
    struct LsbFirstThenMsbFirst: 8 {}

    #[msb_first]
    #[lsb_first]
    struct MsbFirstThenLsbFirst: 8 {}
}

fn main() {}
