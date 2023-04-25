#![allow(arithmetic_overflow)]
#![cfg(test)]

use std::panic::catch_unwind;

use bitint::prelude::*;

#[test]
fn test_profile() {
    if let Err(_) = catch_unwind(|| 255u8 + 1u8) {
        panic!("this crate expects to be tested with overflow-checks disabled");
    }
}

#[bitint_literals]
#[test]
fn test_bitint_add_overflow_in_primitive_op_wraps() {
    assert_eq!(127_U7 + 129, 0_U7);
}

#[bitint_literals]
#[test]
fn test_bitint_add_overflow_in_conversion_wraps() {
    assert_eq!(127_U7 + 1_U7, 0_U7);
}

#[bitint_literals]
#[test]
fn test_bitint_sub_overflow_in_primitive_wraps() {
    assert_eq!(0_U7 - 1_U7, 127_U7);
}
