use mvbitfield::prelude::*;

bitfield! {
    struct MyStruct: 32 {
        a: 16,
        b: 16,
    }
}

fn main() {}
