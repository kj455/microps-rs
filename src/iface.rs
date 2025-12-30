use anyhow::Result;

use crate::device::DeviceIndex;
use crate::protocol::ip::IpAddr;

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
    pub device_index: DeviceIndex,
}

impl IpIface {
    pub fn new(unicast: &str, netmask: &str, device_index: DeviceIndex) -> Result<Self> {
        let unicast_addr = IpAddr::from_str(unicast)?;
        let netmask_addr = IpAddr::from_str(netmask)?;
        let broadcast_addr = (unicast_addr & netmask_addr) | !netmask_addr;

        Ok(IpIface {
            unicast: unicast_addr,
            netmask: netmask_addr,
            broadcast: broadcast_addr,
            device_index,
        })
    }

    pub fn is_destination_match(&self, dst: IpAddr) -> bool {
        dst == self.unicast || dst == self.broadcast || dst == IpAddr::BROADCAST
    }

    pub fn info(&self) -> String {
        format!(
            "unicast={}, netmask={}, broadcast={}",
            self.unicast, self.netmask, self.broadcast
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
