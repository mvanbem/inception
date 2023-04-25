#![allow(arithmetic_overflow)]
#![cfg(test)]

use std::panic::catch_unwind;

use bitint::prelude::*;

#[test]
fn test_profile() {
    if let Ok(_) = catch_unwind(|| 255u8 + 1u8) {
        panic!("this crate expects to be tested with overflow-checks enabled");
    }
}

#[bitint_literals]
#[test]
#[should_panic]
fn test_bitint_add_overflow_in_primitive_op_panics() {
    let _ = 127_U7 + 129;
}

#[bitint_literals]
#[test]
#[should_panic]
fn test_bitint_add_overflow_in_conversion_panics() {
    let _ = 127_U7 + 1_U7;
}

#[bitint_literals]
#[test]
#[should_panic]
fn test_bitint_sub_overflow_in_primitive_panics() {
    let _ = 0_U7 - 1_U7;
}
