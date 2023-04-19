use mvbitfield::prelude::*;

bitfield! {
    #[msb_first]
    pub struct MyStruct: u16 {
        pub high_bit: 1 as bool,
        pub next_two_bits: 2,
        ..,
        pub a_low_bit: 1,
        pub last_three_bits: 3,
    }
}

#[narrow_integer_literals]
#[test]
fn test_with_zeros() {
    assert_eq!(
        MyStruct::zero()
            .with_high_bit(false)
            .with_next_two_bits(0u2)
            .with_a_low_bit(0u1)
            .with_last_three_bits(0u3)
            .as_u16(),
        0,
    );
}

#[narrow_integer_literals]
#[test]
fn test_with_bits() {
    assert_eq!(
        MyStruct::zero().with_high_bit(true).as_u16(),
        0b1_00_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_next_two_bits(2u2).as_u16(),
        0b0_10_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_next_two_bits(1u2).as_u16(),
        0b0_01_000000000_0_000,
    );
    assert_eq!(
        MyStruct::zero().with_a_low_bit(1u1).as_u16(),
        0b0_00_000000000_1_000,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(4u3).as_u16(),
        0b0_00_000000000_0_100,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(2u3).as_u16(),
        0b0_00_000000000_0_010,
    );
    assert_eq!(
        MyStruct::zero().with_last_three_bits(1u3).as_u16(),
        0b0_00_000000000_0_001,
    );
}
