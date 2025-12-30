pub mod ip;

use anyhow::Result;

use crate::context::ProtocolContexts;
use crate::device::Device;

pub const PROTOCOL_TYPE_IP: u16 = 0x0800;
pub const PROTOCOL_TYPE_ARP: u16 = 0x0806;
pub const PROTOCOL_TYPE_IPV6: u16 = 0x86dd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    Ip,
    Arp,
    Ipv6,
    Unknown(u16),
}

impl From<u16> for ProtocolType {
    fn from(value: u16) -> Self {
        match value {
            PROTOCOL_TYPE_IP => ProtocolType::Ip,
            PROTOCOL_TYPE_ARP => ProtocolType::Arp,
            PROTOCOL_TYPE_IPV6 => ProtocolType::Ipv6,
            other => ProtocolType::Unknown(other),
        }
    }
}

impl From<ProtocolType> for u16 {
    fn from(value: ProtocolType) -> Self {
        match value {
            ProtocolType::Ip => PROTOCOL_TYPE_IP,
            ProtocolType::Arp => PROTOCOL_TYPE_ARP,
            ProtocolType::Ipv6 => PROTOCOL_TYPE_IPV6,
            ProtocolType::Unknown(v) => v,
        }
    }
}

pub type ProtocolHandler = fn(&[u8], &Device, &ProtocolContexts);

struct Protocol {
    type_: ProtocolType,
    handler: ProtocolHandler,
}

pub struct ProtocolManager {
    protocols: Vec<Protocol>,
}

impl ProtocolManager {
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
        }
    }

    pub fn register(&mut self, type_: ProtocolType, handler: ProtocolHandler) -> Result<()> {
        if self.protocols.iter().any(|p| p.type_ == type_) {
            anyhow::bail!("Protocol already registered: {:?}", type_);
        }

        tracing::debug!("Protocol registered: {:?}", type_);
        self.protocols.push(Protocol { type_, handler });
        Ok(())
    }

    pub fn dispatch(&self, type_: u16, data: &[u8], dev: &Device, ctx: &ProtocolContexts) {
        let protocol_type = ProtocolType::from(type_);

        for protocol in &self.protocols {
            if protocol.type_ == protocol_type {
                (protocol.handler)(data, dev, ctx);
                return;
            }
        }

        tracing::debug!("No handler for protocol type: 0x{:04x}", type_);
    }

    pub fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing protocols...");
        ip::init(self)?;
        tracing::info!("Protocols initialized");
        Ok(())
    }
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}
