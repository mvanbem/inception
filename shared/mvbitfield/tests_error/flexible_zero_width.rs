use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    struct MyStructA: u32 {
        x: 16,
        y: _,
        z: 16,
    }

    #[msb_first]
    struct MyStructB: u32 {
        x: 16,
        ..,
        z: 16,
    }
}

fn main() {}
