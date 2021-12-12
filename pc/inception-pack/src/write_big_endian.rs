use std::io::Write;

use anyhow::Result;
use byteorder::{BigEndian, WriteBytesExt};
use paste::paste;
use seq_macro::seq;

use crate::FloatByBits;

pub trait WriteBigEndian {
    const SIZE: usize;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()>;
}

impl WriteBigEndian for u8 {
    const SIZE: usize = 1;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_u8(*self)?)
    }
}

impl WriteBigEndian for u16 {
    const SIZE: usize = 2;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_u16::<BigEndian>(*self)?)
    }
}

impl WriteBigEndian for u32 {
    const SIZE: usize = 4;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_u32::<BigEndian>(*self)?)
    }
}

impl WriteBigEndian for i8 {
    const SIZE: usize = 1;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_i8(*self)?)
    }
}

impl WriteBigEndian for i16 {
    const SIZE: usize = 2;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_i16::<BigEndian>(*self)?)
    }
}

impl WriteBigEndian for i32 {
    const SIZE: usize = 4;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_i32::<BigEndian>(*self)?)
    }
}

impl WriteBigEndian for f32 {
    const SIZE: usize = 4;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        Ok(w.write_f32::<BigEndian>(*self)?)
    }
}

impl WriteBigEndian for FloatByBits {
    const SIZE: usize = 4;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        self.0.write_big_endian_to(w)
    }
}

impl<T: WriteBigEndian, const N: usize> WriteBigEndian for [T; N] {
    const SIZE: usize = T::SIZE * N;

    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
        for value in self.iter() {
            value.write_big_endian_to(w)?;
        }
        Ok(())
    }
}

macro_rules! impl_write_big_endian_for_tuples {
    () => {
        seq!(N in 1..10 { #(impl_write_big_endian_for_tuples!(N);)* });
    };
    ($n:literal) => {
        seq!(N in 0..$n {
            paste! {
                impl<#([<T N>]: WriteBigEndian,)*> WriteBigEndian for (#([<T N>],)*) {
                    const SIZE: usize = 0 #(+ [<T N>]::SIZE)*;

                    fn write_big_endian_to<W: Write>(&self, w: &mut W) -> Result<()> {
                        #(self.N.write_big_endian_to(w)?;)*
                        Ok(())
                    }
                }
            }
        });
    };
}

impl_write_big_endian_for_tuples!();
