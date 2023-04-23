use mvbitfield::prelude::*;

bitfield! {
    struct MyStruct: 16 {
        x: 16,
    }
}
