use std::collections::HashMap;
use std::io::{self, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};

use byteorder::{BigEndian, WriteBytesExt};

pub struct RelocationWriter<W> {
    inner: W,
    pointers: Vec<Pointer>,
    symbols: HashMap<&'static str, u64>,
}

struct Pointer {
    position: u64,
    format: PointerFormat,
    symbol: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerFormat {
    BigEndianU32,
}

impl<W: Seek + Write> RelocationWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            pointers: Default::default(),
            symbols: Default::default(),
        }
    }

    pub fn write_pointer(&mut self, format: PointerFormat, symbol: &'static str) -> io::Result<()> {
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
        }
        Ok(())
    }

    pub fn define_symbol(&mut self, symbol: &'static str) -> io::Result<()> {
        match self.symbols.insert(symbol, self.inner.stream_position()?) {
            Some(_) => panic!("Duplicate definition of symbol {:?}", symbol),
            None => (),
        }
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<W> {
        let position_to_restore = self.inner.stream_position()?;

        for pointer in self.pointers {
            let symbol_position = self.symbols[pointer.symbol];
            self.inner.seek(SeekFrom::Start(pointer.position))?;
            match pointer.format {
                PointerFormat::BigEndianU32 => self
                    .inner
                    .write_u32::<BigEndian>(u32::try_from(symbol_position).unwrap())?,
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
