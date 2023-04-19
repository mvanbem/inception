use mvbitfield::prelude::*;

struct UserType(u8);

bitfield! {
    struct MyStruct: UserType {
        x: 8,
    }

    struct MyStruct: a::path::instead {
        x: 8,
    }
}

fn main() {}
