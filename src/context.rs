use anyhow::Result;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::iface::IpIface;
use crate::protocol::ip::IpAddr;

pub struct IpIdManager {
    next_id: AtomicU16,
}

impl IpIdManager {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU16::new(1),
        }
    }

    pub fn next(&self) -> u16 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for IpIdManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry of IP interfaces (equivalent to C's `static struct ip_iface *ifaces`)
#[derive(Default)]
pub struct IpIfaceRegistry {
    ifaces: Vec<IpIface>,
}

impl IpIfaceRegistry {
    pub fn new() -> Self {
        Self { ifaces: Vec::new() }
    }

    /// Register an IP interface
    pub fn register(&mut self, iface: IpIface) -> Result<()> {
        // check for duplicates could be added here
        if self
            .ifaces
            .iter()
            .any(|existing| existing.unicast == iface.unicast)
        {
            anyhow::bail!("IP interface with address {} already exists", iface.unicast);
        }

        self.ifaces.push(iface);
        Ok(())
    }

    /// Select an interface by unicast address (equivalent to C's `ip_iface_select`)
    pub fn select(&self, addr: IpAddr) -> Option<&IpIface> {
        self.ifaces.iter().find(|iface| iface.unicast == addr)
    }
}

#[derive(Default)]
pub struct ProtocolContexts {
    pub ip_id: IpIdManager,
    pub ip_ifaces: IpIfaceRegistry,
}

impl ProtocolContexts {
    pub fn new() -> Self {
        Self::default()
    }
}
