#![no_std]

#[derive(Debug)]
pub enum NetError {
    Disconnected,
    Unexpected { function: &'static str, ret: i32 },
}

pub trait Read {
    fn read(&self, buf: &mut [u8]) -> Result<usize, NetError>;
}

pub trait Write {
    fn write(&self, buf: &[u8]) -> Result<usize, NetError>;
}

pub trait ReadExt: Read {
    fn read_all(&self, mut buf: &mut [u8]) -> Result<(), NetError> {
        while buf.len() > 0 {
            let n = self.read(buf)?;
            buf = &mut buf[n..];
        }
        Ok(())
    }
}

impl<T: Read> ReadExt for T {}

pub trait WriteExt: Write {
    fn write_all(&self, mut buf: &[u8]) -> Result<(), NetError> {
        while buf.len() > 0 {
            let n = self.write(buf)?;
            buf = &buf[n..];
        }
        Ok(())
    }
}

impl<T: Write> WriteExt for T {}
