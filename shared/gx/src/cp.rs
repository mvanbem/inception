use crate::common::{MatrixRegA, MatrixRegB};

pub trait CpReg {
    type T: Into<u32>;

    fn addr(&self) -> u8;
}

pub struct CpMatrixRegA;

impl CpReg for CpMatrixRegA {
    type T = MatrixRegA;

    fn addr(&self) -> u8 {
        0x30
    }
}

pub struct CpMatrixRegB;

impl CpReg for CpMatrixRegB {
    type T = MatrixRegB;

    fn addr(&self) -> u8 {
        0x40
    }
}
