use std::io::Seek;
use std::ops::{Deref, DerefMut};

use anyhow::{bail, Result};

pub struct RecordWriter<W: Seek> {
    w: W,
    record_size: u64,
}

impl<W: Seek> RecordWriter<W> {
    pub fn new(w: W, record_size: u64) -> Self {
        Self { w, record_size }
    }

    pub fn index(&mut self) -> Result<u64> {
        let pos = self.stream_position()?;
        let remainder = pos % self.record_size;
        if remainder != 0 {
            bail!(
                "Not on a record boundary: size={}, pos={}, remainder={}",
                self.record_size,
                pos,
                remainder,
            );
        }
        Ok(pos / self.record_size)
    }
}

impl<W: Seek> Deref for RecordWriter<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.w
    }
}

impl<W: Seek> DerefMut for RecordWriter<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.w
    }
}
