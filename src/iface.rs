use anyhow::Result;

use crate::protocol::ip::{ip_addr_ntop, ip_addr_pton, IpAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetIfaceFamily {
    Ip = 1,
    Ipv6 = 2,
}

#[derive(Debug, Clone)]
pub struct IpIface {
    pub unicast: IpAddr,
    pub netmask: IpAddr,
    pub broadcast: IpAddr,
}

impl IpIface {
    pub fn new(unicast: &str, netmask: &str) -> Result<Self> {
        let unicast_addr = ip_addr_pton(unicast)?;
        let netmask_addr = ip_addr_pton(netmask)?;
        let broadcast_addr = (unicast_addr & netmask_addr) | !netmask_addr;

        Ok(IpIface {
            unicast: unicast_addr,
            netmask: netmask_addr,
            broadcast: broadcast_addr,
        })
    }

    pub fn is_destination_match(&self, dst: IpAddr) -> bool {
        dst == self.unicast || dst == self.broadcast || dst == IpAddr::BROADCAST
    }

    pub fn info(&self) -> String {
        format!(
            "unicast={}, netmask={}, broadcast={}",
            ip_addr_ntop(self.unicast),
            ip_addr_ntop(self.netmask),
            ip_addr_ntop(self.broadcast)
        )
    }
}

#[derive(Debug, Clone)]
pub enum NetIface {
    Ip(IpIface),
}

impl NetIface {
    pub fn family(&self) -> NetIfaceFamily {
        match self {
            NetIface::Ip(_) => NetIfaceFamily::Ip,
        }
    }

    pub fn as_ip(&self) -> Option<&IpIface> {
        match self {
            NetIface::Ip(iface) => Some(iface),
        }
    }
}
