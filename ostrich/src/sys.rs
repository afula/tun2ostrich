use super::common;
use super::option;
use log::*;
pub struct NetInfo {
    pub default_ipv4_gateway: Option<String>,
    pub default_ipv6_gateway: Option<String>,
    pub default_ipv4_address: Option<String>,
    pub default_ipv6_address: Option<String>,
    pub ipv4_forwarding: bool,
    pub ipv6_forwarding: bool,
    pub default_interface: Option<String>,
}

impl Default for NetInfo {
    fn default() -> Self {
        Self {
            default_ipv4_gateway: None,
            default_ipv6_gateway: None,
            default_ipv4_address: None,
            default_ipv6_address: None,
            ipv4_forwarding: false,
            ipv6_forwarding: false,
            default_interface: None,
        }
    }
}

pub fn get_net_info() -> NetInfo {
    let iface = common::cmd::get_default_interface().unwrap();
    trace!("#1 iface {:?}", iface);

    let ipv4_gw = common::cmd::get_default_ipv4_gateway().unwrap();
    trace!("#2 ipv4_gw {:?}", ipv4_gw);
    let ipv6_gw = if *option::ENABLE_IPV6 {
        Some(common::cmd::get_default_ipv6_gateway().unwrap())
    } else {
        None
    };
    trace!("#3 ipv6_gw {:?}", ipv6_gw);

    let all_interfaces = pnet_datalink::interfaces();
    let ipv4_addr = if let Some(ifa) = all_interfaces
        .iter()
        .find(|ifa| ifa.name == iface && !ifa.ips.is_empty())
    {
        ifa.ips
            .iter()
            .find(|ipn| ipn.is_ipv4())
            .map(|ipn| ipn.ip().to_string())
    } else {
        None
    };
    trace!("#4 ipv4_addr {:?}", ipv4_addr);
    let ipv6_addr = if *option::ENABLE_IPV6 {
        if let Some(ifa) = all_interfaces
            .iter()
            .find(|ifa| ifa.name == iface && !ifa.ips.is_empty())
        {
            ifa.ips
                .iter()
                .find(|ipn| ipn.is_ipv6())
                .map(|ipn| ipn.ip().to_string())
        } else {
            None
        }
    } else {
        None
    };
    let ipv4_forwarding = common::cmd::get_ipv4_forwarding().unwrap();
    trace!("#5 ipv4_forwarding {:?}", ipv4_forwarding);
    let ipv6_forwarding = if *option::ENABLE_IPV6 {
        common::cmd::get_ipv6_forwarding().unwrap()
    } else {
        false
    };
    trace!("#6 ipv6_forwarding {:?}", ipv6_forwarding);

    NetInfo {
        default_ipv4_gateway: Some(ipv4_gw),
        default_ipv6_gateway: ipv6_gw,
        default_ipv4_address: ipv4_addr,
        default_ipv6_address: ipv6_addr,
        ipv4_forwarding,
        ipv6_forwarding,
        default_interface: Some(iface),
    }
}

pub fn post_tun_creation_setup(net_info: &NetInfo) {
    if let NetInfo {
        default_ipv4_gateway: Some(ipv4_gw),
        default_ipv6_gateway: ipv6_gw,
        default_ipv4_address: ipv4_addr,
        default_ipv6_address: ipv6_addr,
        ipv4_forwarding,
        ipv6_forwarding,
        default_interface: Some(iface),
    } = net_info
    {
        use std::net::{Ipv4Addr, Ipv6Addr};
        common::cmd::add_interface_ipv4_address(
            &*option::DEFAULT_TUN_NAME,
            (*option::DEFAULT_TUN_IPV4_ADDR)
                .parse::<Ipv4Addr>()
                .unwrap(),
            (*option::DEFAULT_TUN_IPV4_GW).parse::<Ipv4Addr>().unwrap(),
            (*option::DEFAULT_TUN_IPV4_MASK)
                .parse::<Ipv4Addr>()
                .unwrap(),
        )
        .unwrap();
        panic!("oooo");
        info!("#7 DEFAULT_TUN_NAME:DEFAULT_TUN_IPV4_ADDR:DEFAULT_TUN_IPV4_GW:DEFAULT_TUN_IPV4_MASK {:?}:{:?}:{:?}:{:?}",&*option::DEFAULT_TUN_NAME,&*option::DEFAULT_TUN_IPV4_ADDR,&*option::DEFAULT_TUN_IPV4_GW,&*option::DEFAULT_TUN_IPV4_MASK);
        // common::cmd::delete_default_ipv4_route(None).unwrap();

        // common::cmd::add_default_ipv4_route(
        //     option::DEFAULT_TUN_IPV4_GW.parse::<Ipv4Addr>().unwrap(),
        //     iface.clone(),
        //     true,
        // )
        // .unwrap();
        // common::cmd::add_default_ipv4_route(
        //     ipv4_gw.parse::<Ipv4Addr>().unwrap(),
        //     iface.clone(),
        //     false,
        // )
        // .unwrap();

        #[cfg(target_os = "linux")]
        {
            if let Some(a) = ipv4_addr {
                common::cmd::add_default_ipv4_rule(a.parse::<Ipv4Addr>().unwrap()).unwrap();
            }
        }

        if *option::GATEWAY_MODE && !ipv4_forwarding {
            common::cmd::set_ipv4_forwarding(true).unwrap();
        }

        if *option::ENABLE_IPV6 {
            common::cmd::add_interface_ipv6_address(
                &*option::DEFAULT_TUN_NAME,
                option::DEFAULT_TUN_IPV6_ADDR.parse::<Ipv6Addr>().unwrap(),
                *option::DEFAULT_TUN_IPV6_PREFIXLEN,
            )
            .unwrap();

            if let Some(ipv6_gw) = ipv6_gw {
                common::cmd::delete_default_ipv6_route(None).unwrap();
                common::cmd::add_default_ipv6_route(
                    option::DEFAULT_TUN_IPV6_GW.parse::<Ipv6Addr>().unwrap(),
                    iface.clone(),
                    true,
                )
                .unwrap();
                common::cmd::add_default_ipv6_route(
                    ipv6_gw.parse::<Ipv6Addr>().unwrap(),
                    iface.clone(),
                    false,
                )
                .unwrap();
            }

            #[cfg(target_os = "linux")]
            {
                if let Some(a) = ipv6_addr {
                    common::cmd::add_default_ipv6_rule(a.parse::<Ipv6Addr>().unwrap()).unwrap();
                }
            }

            if *option::GATEWAY_MODE && !ipv6_forwarding {
                common::cmd::set_ipv6_forwarding(true).unwrap();
            }
        }

        #[cfg(target_os = "linux")]
        {
            if *option::GATEWAY_MODE {
                common::cmd::add_iptable_forward(&*option::DEFAULT_TUN_NAME).unwrap();
            }
        }
    }
}

pub fn post_tun_completion_setup(net_info: &NetInfo) {
    if let NetInfo {
        default_ipv4_gateway: Some(ipv4_gw),
        default_ipv6_gateway: ipv6_gw,
        default_ipv4_address: ipv4_addr,
        default_ipv6_address: ipv6_addr,
        ipv4_forwarding,
        ipv6_forwarding,
        default_interface: Some(iface),
    } = &net_info
    {
        use std::net::{Ipv4Addr, Ipv6Addr};
        // common::cmd::delete_default_ipv4_route(None).unwrap();
        // common::cmd::delete_default_ipv4_route(Some(iface.clone())).unwrap();

        // common::cmd::add_default_ipv4_route(
        //     ipv4_gw.parse::<Ipv4Addr>().unwrap(),
        //     iface.clone(),
        //     true,
        // )
        // .unwrap();
        // 0.0.0.0          0.0.0.0        172.7.0.1
        // #[cfg(target_os = "windows")]{
        //     let out = std::process::Command::new("route")
        //     .arg("delete")
        //     .arg("0.0.0.0")
        //     .arg("172.7.0.1")
        //     .status()
        //     .expect("failed to execute command");
        // println!("process finished with: {}", out);
        // }

        #[cfg(target_os = "linux")]
        {
            if let Some(a) = ipv4_addr {
                common::cmd::delete_default_ipv4_rule(a.parse::<Ipv4Addr>().unwrap()).unwrap();
            }
        }

        if *option::GATEWAY_MODE && !ipv4_forwarding {
            common::cmd::set_ipv4_forwarding(false).unwrap();
        }

        if *option::ENABLE_IPV6 {
            if let Some(ipv6_gw) = ipv6_gw {
                common::cmd::delete_default_ipv6_route(None).unwrap();
                common::cmd::delete_default_ipv6_route(Some(iface.clone())).unwrap();
                common::cmd::add_default_ipv6_route(
                    ipv6_gw.parse::<Ipv6Addr>().unwrap(),
                    iface.clone(),
                    true,
                )
                .unwrap();
            }

            #[cfg(target_os = "linux")]
            {
                if let Some(a) = ipv6_addr {
                    common::cmd::delete_default_ipv6_rule(a.parse::<Ipv6Addr>().unwrap()).unwrap();
                }
            }

            if *option::GATEWAY_MODE && !ipv6_forwarding {
                common::cmd::set_ipv6_forwarding(false).unwrap();
            }
        }

        #[cfg(target_os = "linux")]
        {
            if *option::GATEWAY_MODE {
                common::cmd::delete_iptable_forward(&*option::DEFAULT_TUN_NAME).unwrap();
            }
        }
    }
}
