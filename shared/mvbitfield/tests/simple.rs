use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    pub struct MyStruct: 16 {
        pub high_bit: 1 as bool,
        pub next_two_bits: 2,
        ..,
        pub a_low_bit: 1,
        pub last_three_bits: 3,
    }
}

#[bitint_literals]
#[test]
fn test_with_zeros() {
    assert_eq!(
        MyStruct::zero()
            .with_high_bit(false)
            .with_next_two_bits(0_U2)
            .with_a_low_bit(0_U1)
            .with_last_three_bits(0_U3)
            .to_primitive(),
        0,
    );
}

#[bitint_literals]
#[test]
fn test_with_bits() {
    assert_eq!(
        MyStruct::zero().with_high_bit(true).to_primitive(),
        0b1_00_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_next_two_bits(2_U2).to_primitive(),
        0b0_10_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_next_two_bits(1_U2).to_primitive(),
        0b0_01_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_a_low_bit(1_U1).to_primitive(),
        0b0_00_000000000_1_000,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(4_U3).to_primitive(),
        0b0_00_000000000_0_100,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(2_U3).to_primitive(),
        0b0_00_000000000_0_010,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(1_U3).to_primitive(),
        0b0_00_000000000_0_001,
    );
}

#[bitint_literals]
#[test]
fn test_map() {
    assert_eq!(
        MyStruct::zero()
            .with_high_bit(true)
            .with_next_two_bits(0_U2)
            .map_next_two_bits(|old| {
                assert_eq!(old, 0_U2);
                3_U2
            })
            .to_primitive(),
        0b1_11_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero()
            .with_high_bit(true)
            .with_next_two_bits(3_U2)
            .map_next_two_bits(|old| {
                assert_eq!(old, 3_U2);
                0_U2
            })
            .to_primitive(),
        0b1_00_000000000_0_000,
    );
}

#[bitint_literals]
#[test]
fn test_set() {
    let mut value = MyStruct::zero()
        .with_high_bit(true)
        .with_next_two_bits(0_U2);
    value.set_next_two_bits(3_U2);
    assert_eq!(value.to_primitive(), 0b1_11_000000000_0_000);

    value.set_next_two_bits(0_U2);
    assert_eq!(value.to_primitive(), 0b1_00_000000000_0_000);
}

#[bitint_literals]
#[test]
fn test_replace() {
    let mut value = MyStruct::zero()
        .with_high_bit(true)
        .with_next_two_bits(0_U2);
    assert_eq!(value.replace_next_two_bits(3_U2), 0_U2);
    assert_eq!(value.to_primitive(), 0b1_11_000000000_0_000);

    assert_eq!(value.replace_next_two_bits(0_U2), 3_U2);
    assert_eq!(value.to_primitive(), 0b1_00_000000000_0_000);
}

#[bitint_literals]
#[test]
fn test_update() {
    let mut value = MyStruct::zero()
        .with_high_bit(true)
        .with_next_two_bits(0_U2);
    assert_eq!(
        value.update_next_two_bits(|old| {
            assert_eq!(old, 0_U2);
            3_U2
        }),
        0_U2,
    );
    assert_eq!(value.to_primitive(), 0b1_11_000000000_0_000);

    assert_eq!(
        value.update_next_two_bits(|old| {
            assert_eq!(old, 3_U2);
            0_U2
        }),
        3_U2,
    );
    assert_eq!(value.to_primitive(), 0b1_00_000000000_0_000);
}
