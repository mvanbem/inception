#![no_std]

#[cfg(test)]
extern crate std;

use no_std_io::{NetError, Read, Write, WriteExt};

use crate::buffer::Buffer;

mod buffer;

pub struct FtpClient<S> {
    stream: S,
    response_buffer: Buffer<256>,
    response_parser: FtpResponseParser,
}

impl<S: Read + Write> FtpClient<S> {
    pub fn new(stream: S) -> Result<Self, NetError> {
        let mut client = Self {
            stream,
            response_buffer: Buffer::new(),
            response_parser: FtpResponseParser::new(),
        };
        match client.read_response()? {
            FtpResponse::Code(220) => (), // Service ready for new user.
            resp => panic!("{:?}", resp),
        }
        Ok(client)
    }

    pub fn send(&mut self, command: &[u8]) -> Result<FtpResponse, NetError> {
        self.stream.write_all(command)?;
        self.read_response()
    }

    fn read_response(&mut self) -> Result<FtpResponse, NetError> {
        // Read until the response is complete.
        loop {
            self.response_buffer
                .try_fill_if_empty(|buf| self.stream.read(buf))?;
            let (n, response_code) = self.response_parser.parse(self.response_buffer.get());
            self.response_buffer.consume(n);
            if let Some(response_code) = response_code {
                // Successful end of response.
                return Ok(response_code);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FtpResponse {
    Code(u32),
    FileSize { size: usize },
    EnteringPassiveMode { addr: [u8; 4], port: u16 },
}

#[derive(Debug)]
enum FtpResponseParser {
    ResponseCode { code: u32 },
    AwaitCr { code: u32 },
    AwaitLf { code: u32 },

    FileSizeSize { size: usize },
    FileSizeAwaitLf { size: usize },

    PassiveAwaitLParen,
    PassiveAddr1 { byte: u8 },
    PassiveAddr2 { addr: [u8; 1], byte: u8 },
    PassiveAddr3 { addr: [u8; 2], byte: u8 },
    PassiveAddr4 { addr: [u8; 3], byte: u8 },
    PassivePort1 { addr: [u8; 4], byte: u8 },
    PassivePort2 { addr: [u8; 4], port: u8, byte: u8 },
    PassiveAwaitDot { addr: [u8; 4], port: u16 },
    PassiveAwaitCr { addr: [u8; 4], port: u16 },
    PassiveAwaitLf { addr: [u8; 4], port: u16 },
}

impl FtpResponseParser {
    fn new() -> Self {
        Self::ResponseCode { code: 0 }
    }

    fn parse(&mut self, mut buf: &[u8]) -> (usize, Option<FtpResponse>) {
        let mut n = 0;
        while buf.len() > 0 {
            let b = buf[0];
            buf = &buf[1..];
            n += 1;

            match (&mut *self, b) {
                // Response code parsing.

                // A digit is accumulated into the response code parsed so far.
                (&mut Self::ResponseCode { code }, b) if b.is_ascii_digit() => {
                    let code = 10 * code + (b - b'0') as u32;
                    *self = Self::ResponseCode { code };
                }

                // A space locks in the response code and changes state.
                (&mut Self::ResponseCode { code }, b' ') => {
                    *self = match code {
                        213 => Self::FileSizeSize { size: 0 },
                        227 => Self::PassiveAwaitLParen,
                        code => Self::AwaitCr { code },
                    }
                }

                // Any number of non-<CR> characters are skipped until <CR> is found.
                (&mut Self::AwaitCr { code }, b'\r') => *self = Self::AwaitLf { code },
                (&mut Self::AwaitCr { .. }, _) => (),

                // The next character *must* be <LF>. This completes the response.
                (&mut Self::AwaitLf { code }, b'\n') => {
                    *self = Self::ResponseCode { code: 0 };
                    return (n, Some(FtpResponse::Code(code)));
                }

                // File size parsing.

                // A digit is accumulated into the size parsed so far.
                (&mut Self::FileSizeSize { size }, b) if b.is_ascii_digit() => {
                    let size = 10 * size + (b - b'0') as usize;
                    *self = Self::FileSizeSize { size };
                }
                (&mut Self::FileSizeSize { size }, b'\r') => *self = Self::FileSizeAwaitLf { size },

                // The next character *must* be <LF>. This completes the response.
                (&mut Self::FileSizeAwaitLf { size }, b'\n') => {
                    *self = Self::ResponseCode { code: 0 };
                    return (n, Some(FtpResponse::FileSize { size }));
                }

                // Entering passive mode parsing.

                // Any number of non-'(' characters are skipped until '(' is found.
                (&mut Self::PassiveAwaitLParen, b'(') => *self = Self::PassiveAddr1 { byte: 0 },
                (&mut Self::PassiveAwaitLParen, _) => (),

                // Parse the host:port.
                (&mut Self::PassiveAddr1 { byte }, b) if b.is_ascii_digit() => {
                    let byte = 10 * byte + (b - b'0');
                    *self = Self::PassiveAddr1 { byte };
                }
                (&mut Self::PassiveAddr1 { byte }, b',') => {
                    *self = Self::PassiveAddr2 {
                        addr: [byte],
                        byte: 0,
                    };
                }
                (&mut Self::PassiveAddr2 { addr, byte }, b) if b.is_ascii_digit() => {
                    let byte = 10 * byte + (b - b'0');
                    *self = Self::PassiveAddr2 { addr, byte };
                }
                (&mut Self::PassiveAddr2 { addr, byte }, b',') => {
                    *self = Self::PassiveAddr3 {
                        addr: [addr[0], byte],
                        byte: 0,
                    };
                }
                (&mut Self::PassiveAddr3 { addr, byte }, b) if b.is_ascii_digit() => {
                    let byte = 10 * byte + (b - b'0');
                    *self = Self::PassiveAddr3 { addr, byte };
                }
                (&mut Self::PassiveAddr3 { addr, byte }, b',') => {
                    *self = Self::PassiveAddr4 {
                        addr: [addr[0], addr[1], byte],
                        byte: 0,
                    };
                }
                (&mut Self::PassiveAddr4 { addr, byte }, b) if b.is_ascii_digit() => {
                    let byte = 10 * byte + (b - b'0');
                    *self = Self::PassiveAddr4 { addr, byte };
                }
                (&mut Self::PassiveAddr4 { addr, byte }, b',') => {
                    *self = Self::PassivePort1 {
                        addr: [addr[0], addr[1], addr[2], byte],
                        byte: 0,
                    };
                }
                (&mut Self::PassivePort1 { addr, byte }, b) if b.is_ascii_digit() => {
                    let byte = 10 * byte + (b - b'0');
                    *self = Self::PassivePort1 { addr, byte };
                }
                (&mut Self::PassivePort1 { addr, byte }, b',') => {
                    *self = Self::PassivePort2 {
                        addr,
                        port: byte,
                        byte: 0,
                    };
                }
                (&mut Self::PassivePort2 { addr, port, byte }, b) if b.is_ascii_digit() => {
                    let byte = 10 * byte + (b - b'0');
                    *self = Self::PassivePort2 { addr, port, byte };
                }
                (&mut Self::PassivePort2 { addr, port, byte }, b')') => {
                    *self = Self::PassiveAwaitDot {
                        addr,
                        port: (port as u16) << 8 | byte as u16,
                    };
                }
                (&mut Self::PassiveAwaitDot { addr, port }, b'.') => {
                    *self = Self::PassiveAwaitCr { addr, port };
                }
                (&mut Self::PassiveAwaitCr { addr, port }, b'\r') => {
                    *self = Self::PassiveAwaitLf { addr, port };
                }
                (&mut Self::PassiveAwaitLf { addr, port }, b'\n') => {
                    *self = Self::ResponseCode { code: 0 };
                    return (n, Some(FtpResponse::EnteringPassiveMode { addr, port }));
                }

                // Anything else is a protocol violation (or a deficiency in this parser).
                // TODO: Support multi-line replies.
                (state, byte) => panic!("bad input: state={:?}, byte=0x{:02x}", state, byte),
            }
        }
        (n, None)
    }
}

#[cfg(test)]
mod tests {
    use super::{FtpResponse, FtpResponseParser};

    #[test]
    fn parse_regular_code() {
        let mut parser = FtpResponseParser::new();
        assert_eq!(
            parser.parse(b"220 (fake ftpd)\r\n"),
            (17, Some(FtpResponse::Code(220))),
        );
    }

    #[test]
    fn parse_passive() {
        let mut parser = FtpResponseParser::new();
        assert_eq!(
            parser.parse(b"227 Entering Passive Mode (12,34,56,78,38,148).\r\n"),
            (
                49,
                Some(FtpResponse::EnteringPassiveMode {
                    addr: [12, 34, 56, 78],
                    port: 9876,
                }),
            )
        );
    }

    #[test]
    fn parse_size() {
        let mut parser = FtpResponseParser::new();
        assert_eq!(
            parser.parse(b"213 12345\r\n"),
            (11, Some(FtpResponse::FileSize { size: 12345 })),
        );
    }
}
