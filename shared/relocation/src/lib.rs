use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};

use byteorder::{BigEndian, WriteBytesExt};

pub struct RelocationWriter<W> {
    inner: W,
    pointers: Vec<Pointer>,
    symbols: HashMap<Cow<'static, str>, u64>,
}

struct Pointer {
    position: u64,
    format: PointerFormat,
    symbol: Cow<'static, str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerFormat {
    BigEndianU32,
    BigEndianU24,
}

impl<W: Seek + Write> RelocationWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            pointers: Default::default(),
            symbols: Default::default(),
        }
    }

    pub fn write_pointer(
        &mut self,
        format: PointerFormat,
        symbol: Cow<'static, str>,
    ) -> io::Result<()> {
        match format {
            PointerFormat::BigEndianU32 => {
                let position = self.inner.stream_position()?;
                self.inner.write_u32::<BigEndian>(0)?;
                self.pointers.push(Pointer {
                    position,
                    format,
                    symbol,
                });
            }
            PointerFormat::BigEndianU24 => {
                let position = self.inner.stream_position()?;
                self.inner.write_u8(0)?;
                self.inner.write_u8(0)?;
                self.inner.write_u8(0)?;
                self.pointers.push(Pointer {
                    position,
                    format,
                    symbol,
                });
            }
        }
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the given symbol has already been defined.
    pub fn define_symbol(&mut self, symbol: Cow<'static, str>, value: u64) {
        if self.symbols.insert(symbol.clone(), value).is_some() {
            panic!("Duplicate definition of symbol {:?}", symbol);
        }
    }

    /// # Panics
    ///
    /// Panics if the given symbol has already been defined.
    pub fn define_symbol_here(&mut self, symbol: Cow<'static, str>) -> io::Result<()> {
        let value = self.inner.stream_position()?;
        self.define_symbol(symbol, value);
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<W> {
        let position_to_restore = self.inner.stream_position()?;

        for pointer in self.pointers {
            let symbol_position = match self.symbols.get(&pointer.symbol) {
                Some(&x) => x,
                None => panic!("Undefined symbol {:?}", pointer.symbol),
            };
            self.inner.seek(SeekFrom::Start(pointer.position))?;
            match pointer.format {
                PointerFormat::BigEndianU32 => self
                    .inner
                    .write_u32::<BigEndian>(u32::try_from(symbol_position).unwrap())?,
                PointerFormat::BigEndianU24 => {
                    let value = u32::try_from(symbol_position).unwrap();
                    assert_eq!(value & 0xff000000, 0);
                    self.inner.write_u8((value >> 16) as u8)?;
                    self.inner.write_u8((value >> 8) as u8)?;
                    self.inner.write_u8(value as u8)?;
                }
            }
        }

        self.inner.seek(SeekFrom::Start(position_to_restore))?;
        Ok(self.inner)
    }
}

impl<W> Deref for RelocationWriter<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W> DerefMut for RelocationWriter<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
