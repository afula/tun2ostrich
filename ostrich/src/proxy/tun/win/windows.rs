// use simple_wintun::adapter::{WintunAdapter, WintunStream};
// use simple_wintun::ReadResult;
use super::TunIpAddr;
use std::io::{Error, ErrorKind, Result};
use std::net::Ipv4Addr;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use wintun::{Adapter, Session};
// #[derive(Clone)]
// pub struct TunIpAddr {
//     pub ip: Ipv4Addr,
//     pub netmask: Ipv4Addr,
// }

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
//     Wintun::create(mtu, ip_addrs)
// }

const ADAPTER_NAME: &str = "utun233";
const TUNNEL_TYPE: &str = "proxy";
const ADAPTER_GUID: &str = "{248B1B2B-94FA-0E20-150F-5C2D2FB4FBF9}";
//1MB
const ADAPTER_BUFF_SIZE: u32 = 1048576;

pub struct Wintun {
    pub session: Arc<Session>,
    _adapter: Arc<Adapter>,
}

impl Wintun {
    pub fn create(mtu: usize, ip_addrs: &[TunIpAddr]) -> Result<Wintun> {
        // drop old wintun adapter
        // {
        //     let _ = WintunAdapter::open_adapter(ADAPTER_NAME);
        // }

        //try to fix the stuck
        // std::thread::sleep(Duration::from_millis(100));
        // let adapter = WintunAdapter::create_adapter(ADAPTER_NAME, TUNNEL_TYPE, ADAPTER_GUID)?;

        let wintun = unsafe { wintun::load_from_path("wintun.dll").expect("cant load dll") };
        let adapter =
            Adapter::create(&wintun, "utun233", ADAPTER_NAME, None).expect("cant create adapter");
        let session = Arc::new(
            adapter
                .start_session(wintun::MAX_RING_CAPACITY)
                .expect("cant start adapter session"),
        );

        // for TunIpAddr { ip, netmask } in ip_addrs {
        //     let status = Command::new("netsh")
        //         .args([
        //             "interface",
        //             "ip",
        //             "add",
        //             "address",
        //             ADAPTER_NAME,
        //             ip.to_string().as_str(),
        //             netmask.to_string().as_str(),
        //         ])
        //         .output()?
        //         .status;

        //     if !status.success() {
        //         return Err(Error::new(ErrorKind::Other, "Failed to add tun ip address"));
        //     }
        // }

        // let status = Command::new("netsh")
        //     .args([
        //         "interface",
        //         "ipv4",
        //         "set",
        //         "subinterface",
        //         ADAPTER_NAME,
        //         &format!("mtu={}", mtu),
        //         "store=persistent",
        //     ])
        //     .output()?
        //     .status;

        // if !status.success() {
        //     return Err(Error::new(ErrorKind::Other, "Failed to set tun mtu"));
        // }

        // TODO self reference
        // let session: WintunStream<'static> =
        //     unsafe { std::mem::transmute(adapter.start_session(ADAPTER_BUFF_SIZE)?) };
        Ok(Wintun {
            session,
            _adapter: adapter,
        })
    }
}

// impl TunDevice for Wintun {
//     fn send_packet(&self, packet: &[u8]) -> Result<()> {
//         self.session.write_packet(packet)
//     }

//     fn recv_packet(&self, buff: &mut [u8]) -> Result<usize> {
//         // let res = self.session.read_packet(buff)?;

//         // match res {
//         //     ReadResult::Success(len) => Ok(len),
//         //     ReadResult::NotEnoughSize(_) => Ok(0),
//         // }

//         match self.session.receive_blocking() {
//             Ok(mut packet) => {
//                 buff.
//                 Ok(packet.bytes().len())
//             }
//             Err(err) => {
//                 error!("Got error while reading: {:?}", err);
//                 Err(err)
//             }
//         }

//     }
// }
