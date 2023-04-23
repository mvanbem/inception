use mvbitfield::prelude::*;

bitfield! {
    struct MyStructA: 0 {
        x: 8,
    }

    struct MyStructB: 129 {
        x: 8,
    }

    struct MyStructC: 680564733841876926926749214863536422912 {
        x: 8,
    }

    struct MyStructD: -1 {
        x: 8,
    }
}

fn main() {}
