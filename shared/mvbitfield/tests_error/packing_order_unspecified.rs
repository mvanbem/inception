use mvbitfield::prelude::*;

bitfield! {
    struct MyStruct: u32 {
        a: 16,
        b: 16,
    }
}

fn main() {}
