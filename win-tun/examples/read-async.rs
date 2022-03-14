//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//                    Version 2, December 2004
//
// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// Everyone is permitted to copy and distribute verbatim or modified
// copies of this license document, and changing it is allowed as long
// as the name is changed.
//
//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
//
//  0. You just DO WHAT THE FUCK YOU WANT TO.

use futures::StreamExt;
use packet::ip::Packet;

#[tokio::main]
async fn main() {
    let mut config = tun::Configuration::default();

    config
        .address((10, 0, 0, 2))
        .netmask((255, 255, 255, 0))
        .up();

    #[cfg(target_os = "linux")]
    config.platform(|config| {
        config.packet_information(true);
    });

    let dev = tun::create_as_async(&config).unwrap();

    let mut stream = dev.into_framed();

    while let Some(packet) = stream.next().await {
        match packet {
            Ok(pkt) => {
                // std::io::copy(&mut pkt.get_bytes(),&mut buf );
                let buf = pkt.get_bytes();
                // println!("pkt: {:#?}", buf);
                println!("pkt: {:#?}", Packet::unchecked(buf));
                // println!("pkt: len {}",buf.len());
            }
            Err(err) => panic!("Error: {:?}", err),
        }
    }
}
