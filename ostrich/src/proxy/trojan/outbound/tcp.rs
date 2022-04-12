use std::convert::TryFrom;
use std::io;

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use sha2::{Digest, Sha224};
use tokio::io::AsyncWriteExt;

use crate::{
    proxy::*,
    session::{Session, SocksAddrWireType},
};

use {
    std::sync::Arc,
    tokio_rustls::{rustls::ClientConfig, TlsConnector},
};
fn tls_err<E>(_error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(
        io::ErrorKind::Other,
        format!("tls error: {:?}", _error.into()),
    )
}
pub struct Handler {
    pub address: String,
    pub port: u16,
    pub password: String,

    pub server_name: String,
    pub tls_config: Arc<ClientConfig>,
}

#[async_trait]
impl TcpOutboundHandler for Handler {
    type Stream = AnyStream;

    fn connect_addr(&self) -> Option<OutboundConnect> {
        Some(OutboundConnect::Proxy(self.address.clone(), self.port))
    }

    async fn handle<'a>(
        &'a self,
        sess: &'a Session,
        stream: Option<Self::Stream>,
    ) -> io::Result<Self::Stream> {
        let stream = stream.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "invalid input"))?;

        let name = if !&self.address.is_empty() {
            self.address.clone()
        } else {
            sess.destination.host()
        };

        let config = TlsConnector::from(self.tls_config.clone());
        // let dnsname = DNSNameRef::try_from_ascii_str(&name).map_err(tls_err)?;

        let dnsname = rustls::ServerName::try_from(name.as_str()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid dnsname: {:?}", &name),
            )
        })?;

        let mut stream = config.connect(dnsname, stream).map_err(tls_err).await?;

        let mut buf = BytesMut::new();
        let password = Sha224::digest(self.password.as_bytes());
        let password = hex::encode(&password[..]);
        buf.put_slice(password.as_bytes());
        buf.put_slice(b"\r\n");
        buf.put_u8(0x01); // tcp
        sess.destination
            .write_buf(&mut buf, SocksAddrWireType::PortLast);
        buf.put_slice(b"\r\n");
        // FIXME combine header and first payload
        stream.write_all(&buf).await?;
        Ok(Box::new(stream))
    }
}
