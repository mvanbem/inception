use mvbitfield::prelude::*;

mod custom {
    use mvbitfield::prelude::*;

    pub struct PrimitiveCustomField(u8);

    impl From<U8> for PrimitiveCustomField {
        fn from(value: U8) -> Self {
            Self::from_bitint(value)
        }
    }

    impl From<PrimitiveCustomField> for U8 {
        fn from(value: PrimitiveCustomField) -> Self {
            value.to_bitint()
        }
    }

    impl Bitfield for PrimitiveCustomField {
        type BitInt = U8;

        const ZERO: Self = Self(0);

        fn zero() -> Self {
            Self::ZERO
        }

        fn from_bitint(value: U8) -> Self {
            Self(value.to_primitive())
        }

        fn to_bitint(self) -> U8 {
            U8::from_primitive(self.0)
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
        MyStruct::zero().with_x(255u8.into()).to_primitive(),
        0b00000_0_11111111_00,
    );
}
