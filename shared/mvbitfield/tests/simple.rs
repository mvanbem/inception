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
            .with_next_two_bits(0u2)
            .with_a_low_bit(0u1)
            .with_last_three_bits(0u3)
            .to_underlying(),
        0,
    );
}

#[bitint_literals]
#[test]
fn test_with_bits() {
    assert_eq!(
        MyStruct::zero().with_high_bit(true).to_underlying(),
        0b1_00_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_next_two_bits(2u2).to_underlying(),
        0b0_10_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_next_two_bits(1u2).to_underlying(),
        0b0_01_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_a_low_bit(1u1).to_underlying(),
        0b0_00_000000000_1_000,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(4u3).to_underlying(),
        0b0_00_000000000_0_100,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(2u3).to_underlying(),
        0b0_00_000000000_0_010,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(1u3).to_underlying(),
        0b0_00_000000000_0_001,
    );
}

#[bitint_literals]
#[test]
fn test_map() {
    assert_eq!(
        MyStruct::zero()
            .with_high_bit(true)
            .with_next_two_bits(0u2)
            .map_next_two_bits(|old| {
                assert_eq!(old, 0u2);
                3u2
            })
            .to_underlying(),
        0b1_11_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero()
            .with_high_bit(true)
            .with_next_two_bits(3u2)
            .map_next_two_bits(|old| {
                assert_eq!(old, 3u2);
                0u2
            })
            .to_underlying(),
        0b1_00_000000000_0_000,
    );
}

#[bitint_literals]
#[test]
fn test_set() {
    let mut value = MyStruct::zero().with_high_bit(true).with_next_two_bits(0u2);
    value.set_next_two_bits(3u2);
    assert_eq!(value.to_underlying(), 0b1_11_000000000_0_000);

    value.set_next_two_bits(0u2);
    assert_eq!(value.to_underlying(), 0b1_00_000000000_0_000);
}

#[bitint_literals]
#[test]
fn test_replace() {
    let mut value = MyStruct::zero().with_high_bit(true).with_next_two_bits(0u2);
    assert_eq!(value.replace_next_two_bits(3u2), 0u2);
    assert_eq!(value.to_underlying(), 0b1_11_000000000_0_000);

    assert_eq!(value.replace_next_two_bits(0u2), 3u2);
    assert_eq!(value.to_underlying(), 0b1_00_000000000_0_000);
}

#[bitint_literals]
#[test]
fn test_update() {
    let mut value = MyStruct::zero().with_high_bit(true).with_next_two_bits(0u2);
    assert_eq!(
        value.update_next_two_bits(|old| {
            assert_eq!(old, 0u2);
            3u2
        }),
        0u2,
    );
    assert_eq!(value.to_underlying(), 0b1_11_000000000_0_000);

    assert_eq!(
        value.update_next_two_bits(|old| {
            assert_eq!(old, 3u2);
            0u2
        }),
        3u2,
    );
    assert_eq!(value.to_underlying(), 0b1_00_000000000_0_000);
}
