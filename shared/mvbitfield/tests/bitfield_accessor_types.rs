use mvbitfield::prelude::*;

mod custom {
    use mvbitfield::prelude::*;

    pub struct PrimitiveCustomField(u8);

    impl From<u8> for PrimitiveCustomField {
        fn from(value: u8) -> Self {
            Self(value)
        }
    }

    impl From<PrimitiveCustomField> for u8 {
        fn from(value: PrimitiveCustomField) -> Self {
            value.0
        }
    }

    impl Bitfield for PrimitiveCustomField {
        type Underlying = u8;

        const ZERO: Self = Self(0);

        fn zero() -> Self {
            Self::ZERO
        }

        fn from_underlying(value: Self::Underlying) -> Self {
            Self(value)
        }

        fn to_underlying(self) -> Self::Underlying {
            self.0
        }
    }
}

bitfield! {
    #[lsb_first]
    struct MyStruct: 11 {
        _: 2,
        x: 8 as custom::PrimitiveCustomField,
        _: 1,
    }
}

#[bitint_literals]
#[test]
fn test_primitive_custom_field() {
    assert_eq!(
        MyStruct::zero().with_x(255.into()).to_primitive(),
        0b00000_0_11111111_00,
    );
}
