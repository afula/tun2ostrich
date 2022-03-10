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

use std::io::Read;

fn main() {
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
    let mut dev = tun::create(&config).unwrap();
    let mut stream = std::net::TcpStream::connect("127.0.0.1:8081").unwrap();
    let s = std::io::copy(&mut dev, &mut stream);
    println!("{:?} bytes copied", s);
}
