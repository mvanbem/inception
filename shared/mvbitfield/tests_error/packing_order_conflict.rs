use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    #[msb_first]
    struct MsbFirstTwice: u8 {}

    #[lsb_first]
    #[lsb_first]
    struct LsbFirstTwice: u8 {}

    #[lsb_first]
    #[msb_first]
    struct LsbFirstThenMsbFirst: u8 {}

    #[msb_first]
    #[lsb_first]
    struct MsbFirstThenLsbFirst: u8 {}
}

fn main() {}
