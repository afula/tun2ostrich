use std::io::Result;
use std::net::Ipv4Addr;
use std::sync::Arc;

#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Clone)]
pub struct TunIpAddr {
    pub ip: Ipv4Addr,
    pub netmask: Ipv4Addr,
}

// pub trait TunDevice: Send + Sync {
//     fn send_packet(&self, packet: &[u8]) -> Result<()>;

//     fn recv_packet(&self, buff: &mut [u8]) -> Result<usize>;
// }

// impl<T: TunDevice> TunDevice for Arc<T> {
//     fn send_packet(&self, packet: &[u8]) -> Result<()> {
//         (**self).send_packet(packet)
//     }

//     fn recv_packet(&self, buff: &mut [u8]) -> Result<usize> {
//         (**self).recv_packet(buff)
//     }
// }

// pub(crate) fn create_device(mtu: usize, ip_addrs: &[TunIpAddr]) -> Result<impl TunDevice> {
//     #[cfg(target_os = "windows")]
//     {
//         windows::Wintun::create(mtu, ip_addrs)
//     }
//     #[cfg(target_os = "linux")]
//     {
//         linux::Linuxtun::create(mtu, ip_addrs)
//     }
//     #[cfg(target_os = "macos")]
//     {
//         Ok(macos::Mactun::create(address, netmask)?)
//     }
// }
