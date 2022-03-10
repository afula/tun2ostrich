use futures::StreamExt;
use packet::ip::Packet;
use std::future::Future;
use std::io;
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut config = tun::Configuration::default();

    config
        .address((10, 0, 0, 2))
        .netmask((255, 255, 255, 0))
        .destination((10, 0, 0, 1))
        .up();

    #[cfg(target_os = "linux")]
    config.platform(|config| {
        config.packet_information(true);
    });

    let mut dev = tun::create_as_async(&config).unwrap();
    let sock = UdpSocket::bind("0.0.0.0:8082").await?;
    let remote_addr = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    sock.connect(remote_addr).await?;
    let mut outlet = UdpOutlet::new(sock);
    println!("started");
    let (u, d) = tokio::io::copy_bidirectional(&mut outlet, &mut dev).await?;
    println!("up: {}, down: {}", u, d);
    Ok(())
}

struct UdpOutlet {
    inner: UdpSocket,
}

impl UdpOutlet {
    pub fn new(udp: UdpSocket) -> Self {
        Self { inner: udp }
    }
}

impl AsyncRead for UdpOutlet {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.inner.poll_recv_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => self.inner.poll_recv(cx, buf),
        }
    }
}

impl AsyncWrite for UdpOutlet {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.inner.poll_send_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => self.inner.poll_send(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
