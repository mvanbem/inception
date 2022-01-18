use core::alloc::Allocator;

use crate::loader::Loader;
use crate::net::{self, SocketAddr, TcpStream};

use alloc::alloc::Global;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inception_render_common::map_data::MapData;
use no_std_ftp::{FtpClient, FtpResponse};
use no_std_io::{NetError, Read};
use ogc_sys::GlobalAlign32;

pub struct FtpLoader {
    addr: SocketAddr,
}

impl Loader for FtpLoader {
    type Params = SocketAddr;
    type Data = Vec<u8, GlobalAlign32>;

    fn new(addr: Self::Params) -> Self {
        unsafe {
            libc::printf(b"Initializing Broadband Adapter...\n\0".as_ptr());
            net::init().unwrap();

            Self { addr }
        }
    }

    fn maps(&mut self) -> Vec<String> {
        let data = ftp_get(&self.addr, "maps.txt").unwrap();
        let mut maps = Vec::new();
        for line in data.split(|&b| b == b'\n') {
            if line.len() > 0 {
                maps.push(String::from_utf8(line.to_vec()).unwrap());
            }
        }
        maps
    }

    fn load_map(&mut self, map: &str) -> MapData<Self::Data> {
        let data = ftp_get_in(&self.addr, &format!("{}.dat", map), GlobalAlign32).unwrap();
        unsafe { MapData::new(data) }
    }
}

fn ftp_get(addr: &SocketAddr, path: &str) -> Result<Vec<u8>, NetError> {
    ftp_get_in(addr, path, Global)
}

fn ftp_get_in<A: Allocator>(
    addr: &SocketAddr,
    path: &str,
    alloc: A,
) -> Result<Vec<u8, A>, NetError> {
    let stream = TcpStream::connect(addr)?;
    stream.socket().set_no_delay()?;
    let mut client = FtpClient::new(stream)?;

    // Log in anonymously.
    match client.send(b"USER anonymous\r\n")? {
        FtpResponse::Code(230) => (), // User logged in, proceed.
        resp => panic!("Unexpected response to USER: {:?}", resp),
    }

    // Set binary image mode (just bytes on the data connection).
    match client.send(b"TYPE I\r\n")? {
        FtpResponse::Code(200) => (), // Command okay.
        resp => panic!("Unexpected response to TYPE: {:?}", resp),
    }

    // Switch to passive mode and establish the data connection.
    let addr = match client.send(b"PASV\r\n")? {
        FtpResponse::EnteringPassiveMode { addr, port } => SocketAddr::new(addr, port),
        resp => panic!("Unexpected response to PASV: {:?}", resp),
    };
    let data_stream = TcpStream::connect(&addr)?;

    // Retrieve the file.
    // NOTE: This makes no attempt to encode the path correctly. Interesting characters will cause
    // this to fail.
    let command = format!("RETR {}\r\n", path);
    match client.send(command.as_bytes())? {
        FtpResponse::Code(150) => (), // File status okay; about to open data connection.
        resp => panic!("Unexpected response to RETR: {:?}", resp),
    }

    // Read the file from the data connection.
    let mut data = Vec::new_in(alloc);
    let mut bytes_read = 0;
    loop {
        // Reserve a 4K buffer to read into at the end of the existing data.
        data.resize(bytes_read + 4096, 0);
        match data_stream.read(&mut data[bytes_read..])? {
            0 => break,
            n => bytes_read += n,
        }
    }
    data.resize(bytes_read, 0);

    // There should be a response confirming the transfer is complete, but at this point we can just
    // close both connections and declare success.

    Ok(data)
}
