use std::{
    convert::From,
    fmt::{self, Debug, Display, Formatter},
    io::{self, ErrorKind},
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
    slice,
    str::FromStr,
    vec,
};

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[rustfmt::skip]
mod consts {
/*    pub const SOCKS5_VERSION:                          u8 = 0x05;

    pub const SOCKS5_AUTH_METHOD_NONE:                 u8 = 0x00;
    pub const SOCKS5_AUTH_METHOD_GSSAPI:               u8 = 0x01;
    pub const SOCKS5_AUTH_METHOD_PASSWORD:             u8 = 0x02;
    pub const SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE:       u8 = 0xff;

    pub const SOCKS5_CMD_TCP_CONNECT:                  u8 = 0x01;
    pub const SOCKS5_CMD_TCP_BIND:                     u8 = 0x02;
    pub const SOCKS5_CMD_UDP_ASSOCIATE:                u8 = 0x03;*/

    pub const SOCKS5_ADDR_TYPE_IPV4:                   u8 = 0x01;
    pub const SOCKS5_ADDR_TYPE_DOMAIN_NAME:            u8 = 0x03;
    pub const SOCKS5_ADDR_TYPE_IPV6:                   u8 = 0x04;

/*    pub const SOCKS5_REPLY_SUCCEEDED:                  u8 = 0x00;
    pub const SOCKS5_REPLY_GENERAL_FAILURE:            u8 = 0x01;
    pub const SOCKS5_REPLY_CONNECTION_NOT_ALLOWED:     u8 = 0x02;
    pub const SOCKS5_REPLY_NETWORK_UNREACHABLE:        u8 = 0x03;
    pub const SOCKS5_REPLY_HOST_UNREACHABLE:           u8 = 0x04;
    pub const SOCKS5_REPLY_CONNECTION_REFUSED:         u8 = 0x05;
    pub const SOCKS5_REPLY_TTL_EXPIRED:                u8 = 0x06;
    pub const SOCKS5_REPLY_COMMAND_NOT_SUPPORTED:      u8 = 0x07;
    pub const SOCKS5_REPLY_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;*/
}

/// SOCKS5 protocol error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    IoError(#[from] io::Error),
    #[error("address type {0:#x} not supported")]
    AddressTypeNotSupported(u8),
    #[error("address domain name must be UTF-8 encoding")]
    AddressDomainInvalidEncoding,
    #[error("unsupported socks version {0:#x}")]
    UnsupportedSocksVersion(u8),
    #[error("unsupported command {0:#x}")]
    UnsupportedCommand(u8),
    #[error("unsupported username/password authentication version {0:#x}")]
    UnsupportedPasswdAuthVersion(u8),
    #[error("username/password authentication invalid request")]
    PasswdAuthInvalidRequest,
}

impl From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        match err {
            Error::IoError(err) => err,
            e => io::Error::new(ErrorKind::Other, e),
        }
    }
}

/// SOCKS5 address type
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Address {
    /// Socket address (IP Address)
    SocketAddress(SocketAddr),
    /// Domain name address
    DomainNameAddress(String, u16),
}

impl Address {
    /// Parse from a `AsyncRead`
    pub async fn read_from<R>(stream: &mut R) -> Result<Address, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr_type_buf = [0u8; 1];
        let _ = stream.read_exact(&mut addr_type_buf).await?;

        let addr_type = addr_type_buf[0];
        match addr_type {
            consts::SOCKS5_ADDR_TYPE_IPV4 => {
                let mut buf = [0u8; 6];
                let _ = stream.read_exact(&mut buf).await?;

                let v4addr = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
                let port = unsafe {
                    let raw_port = &buf[4..];
                    u16::from_be(*(raw_port.as_ptr() as *const _))
                };
                Ok(Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(
                    v4addr, port,
                ))))
            }
            consts::SOCKS5_ADDR_TYPE_IPV6 => {
                let mut buf = [0u8; 18];
                let _ = stream.read_exact(&mut buf).await?;

                let buf: &[u16] = unsafe { slice::from_raw_parts(buf.as_ptr() as *const _, 9) };

                let v6addr = Ipv6Addr::new(
                    u16::from_be(buf[0]),
                    u16::from_be(buf[1]),
                    u16::from_be(buf[2]),
                    u16::from_be(buf[3]),
                    u16::from_be(buf[4]),
                    u16::from_be(buf[5]),
                    u16::from_be(buf[6]),
                    u16::from_be(buf[7]),
                );
                let port = u16::from_be(buf[8]);

                Ok(Address::SocketAddress(SocketAddr::V6(SocketAddrV6::new(
                    v6addr, port, 0, 0,
                ))))
            }
            consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME => {
                let mut length_buf = [0u8; 1];
                let _ = stream.read_exact(&mut length_buf).await?;
                let length = length_buf[0] as usize;

                // Len(Domain) + Len(Port)
                let buf_length = length + 2;

                let mut raw_addr = vec![0u8; buf_length];
                let _ = stream.read_exact(&mut raw_addr).await?;

                let raw_port = &raw_addr[length..];
                let port = unsafe { u16::from_be(*(raw_port.as_ptr() as *const _)) };

                raw_addr.truncate(length);

                let addr = match String::from_utf8(raw_addr) {
                    Ok(addr) => addr,
                    Err(..) => return Err(Error::AddressDomainInvalidEncoding),
                };

                Ok(Address::DomainNameAddress(addr, port))
            }
            _ => {
                // Wrong Address Type . Socks5 only supports ipv4, ipv6 and domain name
                Err(Error::AddressTypeNotSupported(addr_type))
            }
        }
    }

    /// Writes to writer
    #[inline]
    pub async fn write_to<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        let mut buf = BytesMut::with_capacity(self.serialized_len());
        self.write_to_buf(&mut buf);
        writer.write_all(&buf).await
    }

    /// Writes to buffer
    #[inline]
    pub fn write_to_buf<B: BufMut>(&self, buf: &mut B) {
        write_address(self, buf)
    }

    /// Get required buffer size for serializing
    #[inline]
    pub fn serialized_len(&self) -> usize {
        get_addr_len(self)
    }

    /// Get maximum required buffer size for serializing
    #[inline]
    pub fn max_serialized_len() -> usize {
        1 // ADDR_TYPE
            + 1 // DOMAIN LENGTH
            + u8::MAX as usize // MAX DOMAIN
            + 2 // PORT
    }

    /// Get associated port number
    pub fn port(&self) -> u16 {
        match *self {
            Address::SocketAddress(addr) => addr.port(),
            Address::DomainNameAddress(.., port) => port,
        }
    }

    /// Get host address string
    pub fn host(&self) -> String {
        match *self {
            Address::SocketAddress(ref addr) => addr.ip().to_string(),
            Address::DomainNameAddress(ref domain, ..) => domain.to_owned(),
        }
    }
}

impl Debug for Address {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Address::SocketAddress(ref addr) => write!(f, "{}", addr),
            Address::DomainNameAddress(ref addr, ref port) => write!(f, "{}:{}", addr, port),
        }
    }
}

impl fmt::Display for Address {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Address::SocketAddress(ref addr) => write!(f, "{}", addr),
            Address::DomainNameAddress(ref addr, ref port) => write!(f, "{}:{}", addr, port),
        }
    }
}

impl ToSocketAddrs for Address {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        match self.clone() {
            Address::SocketAddress(addr) => Ok(vec![addr].into_iter()),
            Address::DomainNameAddress(addr, port) => (&addr[..], port).to_socket_addrs(),
        }
    }
}

impl From<SocketAddr> for Address {
    fn from(s: SocketAddr) -> Address {
        Address::SocketAddress(s)
    }
}

impl From<(String, u16)> for Address {
    fn from((dn, port): (String, u16)) -> Address {
        Address::DomainNameAddress(dn, port)
    }
}

impl From<&Address> for Address {
    fn from(addr: &Address) -> Address {
        addr.clone()
    }
}

/// Parse `Address` error
#[derive(Debug)]
pub struct AddressError;

impl Display for AddressError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("invalid Address")
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Address, AddressError> {
        match s.parse::<SocketAddr>() {
            Ok(addr) => Ok(Address::SocketAddress(addr)),
            Err(..) => {
                let mut sp = s.split(':');
                match (sp.next(), sp.next()) {
                    (Some(dn), Some(port)) => match port.parse::<u16>() {
                        Ok(port) => Ok(Address::DomainNameAddress(dn.to_owned(), port)),
                        Err(..) => Err(AddressError),
                    },
                    (Some(dn), None) => {
                        // Assume it is 80 (http's default port)
                        Ok(Address::DomainNameAddress(dn.to_owned(), 80))
                    }
                    _ => Err(AddressError),
                }
            }
        }
    }
}

fn write_ipv4_address<B: BufMut>(addr: &SocketAddrV4, buf: &mut B) {
    buf.put_u8(consts::SOCKS5_ADDR_TYPE_IPV4); // Address type
    buf.put_slice(&addr.ip().octets()); // Ipv4 bytes
    buf.put_u16(addr.port()); // Port
}

fn write_ipv6_address<B: BufMut>(addr: &SocketAddrV6, buf: &mut B) {
    buf.put_u8(consts::SOCKS5_ADDR_TYPE_IPV6); // Address type
    for seg in &addr.ip().segments() {
        buf.put_u16(*seg); // Ipv6 bytes
    }
    buf.put_u16(addr.port()); // Port
}

fn write_domain_name_address<B: BufMut>(dnaddr: &str, port: u16, buf: &mut B) {
    assert!(dnaddr.len() <= u8::MAX as usize);

    buf.put_u8(consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME);
    assert!(
        dnaddr.len() <= u8::MAX as usize,
        "domain name length must be smaller than 256"
    );
    buf.put_u8(dnaddr.len() as u8);
    buf.put_slice(dnaddr[..].as_bytes());
    buf.put_u16(port);
}

fn write_socket_address<B: BufMut>(addr: &SocketAddr, buf: &mut B) {
    match *addr {
        SocketAddr::V4(ref addr) => write_ipv4_address(addr, buf),
        SocketAddr::V6(ref addr) => write_ipv6_address(addr, buf),
    }
}

fn write_address<B: BufMut>(addr: &Address, buf: &mut B) {
    match *addr {
        Address::SocketAddress(ref addr) => write_socket_address(addr, buf),
        Address::DomainNameAddress(ref dnaddr, ref port) => {
            write_domain_name_address(dnaddr, *port, buf)
        }
    }
}

#[inline]
fn get_addr_len(atyp: &Address) -> usize {
    match *atyp {
        Address::SocketAddress(SocketAddr::V4(..)) => 1 + 4 + 2,
        Address::SocketAddress(SocketAddr::V6(..)) => 1 + 8 * 2 + 2,
        Address::DomainNameAddress(ref dmname, _) => 1 + 1 + dmname.len() + 2,
    }
}
