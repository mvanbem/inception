use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    struct MyStructA: 32 {
        x: 16,
        y: _,
        z: 16,
    }

    #[msb_first]
    struct MyStructB: 32 {
        x: 16,
        ..,
        z: 16,
    }
}

fn main() {}
