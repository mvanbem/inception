use mvbitfield::prelude::*;

bitfield! {
    struct MyStruct: u16 {
        x: 16,
    }
}
