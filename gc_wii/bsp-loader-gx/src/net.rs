use core::mem::{size_of, zeroed};
use core::ptr::null_mut;

use libc::c_void;
use no_std_io::{NetError, Read, Write};
use ogc_sys::*;

pub fn init() -> Result<(), NetError> {
    unsafe {
        let ret = if_config(null_mut(), null_mut(), null_mut(), true, 20);
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "if_config",
                ret,
            });
        }
        Ok(())
    }
}

pub struct Socket {
    sock: i32,
}

impl Socket {
    fn new() -> Result<Self, NetError> {
        let ret = unsafe { net_socket(AF_INET, SOCK_STREAM, IPPROTO_IP) };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_socket",
                ret,
            });
        }
        Ok(Self { sock: ret })
    }

    pub fn set_no_delay(&self) -> Result<(), NetError> {
        let opt = 1u32;
        let ret = unsafe {
            net_setsockopt(
                self.sock,
                IPPROTO_TCP,
                TCP_NODELAY,
                &opt as *const u32 as *const c_void,
                size_of::<u32>() as u32,
            )
        };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_setsockopt",
                ret,
            });
        }
        Ok(())
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let ret = unsafe { net_close(self.sock) };
        if ret < 0 {
            panic!("net_close returned {}", ret);
        }
    }
}

#[repr(transparent)]
pub struct SocketAddr(sockaddr_in);

impl SocketAddr {
    pub fn new(addr: [u8; 4], port: u16) -> Self {
        let mut sockaddr = unsafe { zeroed::<sockaddr_in>() };
        sockaddr.sin_family = AF_INET as u8;
        sockaddr.sin_port = port;
        sockaddr.sin_addr.s_addr = (addr[0] as u32) << 24
            | (addr[1] as u32) << 16
            | (addr[2] as u32) << 8
            | addr[3] as u32;
        Self(sockaddr)
    }

    pub fn any(port: u16) -> Self {
        let mut sockaddr = unsafe { zeroed::<sockaddr_in>() };
        sockaddr.sin_family = AF_INET as u8;
        sockaddr.sin_port = port;
        sockaddr.sin_addr.s_addr = INADDR_ANY;
        Self(sockaddr)
    }

    fn as_sockaddr(&self) -> *const sockaddr {
        self as *const SocketAddr as *const sockaddr
    }
}

#[derive(Clone, Copy, Default)]
pub struct SendFlags(pub u32);

impl SendFlags {
    pub const NONBLOCK: Self = Self(O_NONBLOCK);
}

#[derive(Clone, Copy, Default)]
pub struct RecvFlags(pub u32);

impl RecvFlags {
    pub const NONBLOCK: Self = Self(O_NONBLOCK);
}

pub struct TcpStream {
    sock: Socket,
}

impl TcpStream {
    pub fn connect(addr: &SocketAddr) -> Result<Self, NetError> {
        let sock = Socket::new()?;

        let ret = unsafe {
            net_connect(
                sock.sock,
                addr.as_sockaddr() as *mut sockaddr,
                size_of::<sockaddr_in>() as u32,
            )
        };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_connect",
                ret,
            });
        }

        Ok(Self { sock })
    }

    pub fn socket(&self) -> &Socket {
        &self.sock
    }

    pub fn write_with_flags(&self, buf: &[u8], flags: SendFlags) -> Result<usize, NetError> {
        let ret = unsafe {
            net_send(
                self.sock.sock,
                buf.as_ptr() as *mut c_void,
                buf.len() as i32,
                flags.0,
            )
        };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_send",
                ret,
            });
        }
        Ok(ret as usize)
    }

    pub fn write_all_with_flags(&self, mut buf: &[u8], flags: SendFlags) -> Result<(), NetError> {
        while buf.len() > 0 {
            let n = self.write_with_flags(buf, flags)?;
            buf = &buf[n..];
        }
        Ok(())
    }

    pub fn read_with_flags(&self, buf: &mut [u8], flags: RecvFlags) -> Result<usize, NetError> {
        let ret = unsafe {
            net_recv(
                self.sock.sock,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as i32,
                flags.0,
            )
        };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_recv",
                ret,
            });
        }
        Ok(ret as usize)
    }

    pub fn read_all_with_flags(
        &self,
        mut buf: &mut [u8],
        flags: RecvFlags,
    ) -> Result<(), NetError> {
        while buf.len() > 0 {
            let n = self.read_with_flags(buf, flags)?;
            buf = &mut buf[n..];
        }
        Ok(())
    }
}

impl Read for TcpStream {
    fn read(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        self.read_with_flags(buf, Default::default())
    }
}

impl Write for TcpStream {
    fn write(&self, buf: &[u8]) -> Result<usize, NetError> {
        self.write_with_flags(buf, Default::default())
    }
}

pub struct TcpListener {
    sock: Socket,
}

impl TcpListener {
    pub fn bind(addr: &SocketAddr) -> Result<Self, NetError> {
        let sock = Socket::new()?;

        let ret = unsafe {
            net_bind(
                sock.sock,
                addr.as_sockaddr() as *mut sockaddr,
                size_of::<sockaddr_in>() as u32,
            )
        };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_bind",
                ret,
            });
        }

        let ret = unsafe { net_listen(sock.sock, 1) };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_listen",
                ret,
            });
        }

        Ok(Self { sock })
    }

    pub fn socket(&self) -> &Socket {
        &self.sock
    }

    pub fn accept(&self) -> Result<TcpStream, NetError> {
        let mut addr = unsafe { zeroed::<sockaddr_in>() };
        let mut addrlen = 0;
        let ret = unsafe {
            net_accept(
                self.sock.sock,
                &mut addr as *mut sockaddr_in as *mut sockaddr,
                &mut addrlen,
            )
        };
        if ret < 0 {
            return Err(NetError::Unexpected {
                function: "net_accept",
                ret,
            });
        }
        Ok(TcpStream {
            sock: Socket { sock: ret },
        })
    }
}
