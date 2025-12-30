use std::fmt;
use std::ops::{BitAnd, BitOr, Not};

use anyhow::Result;

use super::{ProtocolManager, ProtocolType};
use crate::context::ProtocolContexts;
use crate::device::Device;
use crate::iface::NetIface;
use crate::util::{cksum16, debugdump, ntoh16};

pub const IP_VERSION_IPV4: u8 = 4;

pub const IP_HDR_SIZE_MIN: usize = 20;
pub const IP_HDR_SIZE_MAX: usize = 60;

pub const IP_TOTAL_SIZE_MAX: usize = u16::MAX as usize;
pub const IP_PAYLOAD_SIZE_MAX: usize = IP_TOTAL_SIZE_MAX - IP_HDR_SIZE_MIN;

pub const IP_ADDR_LEN: usize = 4;
pub const IP_ADDR_STR_LEN: usize = 16;

const IP_HDR_FLAG_MF: u16 = 0x2000;
#[allow(dead_code)]
const IP_HDR_FLAG_DF: u16 = 0x4000;
#[allow(dead_code)]
const IP_HDR_FLAG_RF: u16 = 0x8000;
const IP_HDR_OFFSET_MASK: u16 = 0x1fff;

pub const IP_PROTOCOL_ICMP: u8 = 1;
pub const IP_PROTOCOL_TCP: u8 = 6;
pub const IP_PROTOCOL_UDP: u8 = 17;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IpAddr(u32);

impl IpAddr {
    pub const ANY: Self = IpAddr(0x00000000);
    pub const BROADCAST: Self = IpAddr(0xffffffff);

    #[inline]
    pub fn from_ne_bytes(bytes: [u8; 4]) -> Self {
        IpAddr(u32::from_ne_bytes(bytes))
    }

    #[inline]
    pub fn to_ne_bytes(self) -> [u8; 4] {
        self.0.to_ne_bytes()
    }
}

impl BitAnd for IpAddr {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        IpAddr(self.0 & rhs.0)
    }
}

impl BitOr for IpAddr {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        IpAddr(self.0 | rhs.0)
    }
}

impl Not for IpAddr {
    type Output = Self;
    fn not(self) -> Self::Output {
        IpAddr(!self.0)
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IpHdr {
    pub vhl: u8,
    pub tos: u8,
    pub total: u16,
    pub id: u16,
    pub offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub sum: u16,
    pub src: IpAddr,
    pub dst: IpAddr,
}

impl IpHdr {
    pub fn from_bytes(data: &[u8]) -> Option<&Self> {
        if data.len() < IP_HDR_SIZE_MIN {
            return None;
        }
        // SAFETY: We've verified the length is sufficient
        Some(unsafe { &*(data.as_ptr() as *const IpHdr) })
    }

    pub fn version(&self) -> u8 {
        (self.vhl >> 4) & 0x0f
    }

    pub fn hdr_len(&self) -> usize {
        ((self.vhl & 0x0f) as usize) * 4
    }
}

impl fmt::Display for IpHdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "vhl={:#04x}, tos={:#04x}, total={}, id={}, offset={:#06x}, ttl={}, protocol={}, sum={:#06x}, src={}, dst={}",
            self.vhl,
            self.tos,
            u16::from_be(self.total),
            u16::from_be(self.id),
            u16::from_be(self.offset),
            self.ttl,
            self.protocol,
            u16::from_be(self.sum),
            ip_addr_ntop(self.src),
            ip_addr_ntop(self.dst)
        )
    }
}

pub fn ip_addr_pton(s: &str) -> Result<IpAddr> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        anyhow::bail!("Invalid IP address format: {}", s);
    }

    let mut bytes = [0u8; 4];
    for (i, part) in parts.iter().enumerate() {
        let octet: u8 = part
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid octet in IP address: {}", part))?;
        bytes[i] = octet;
    }

    Ok(IpAddr::from_ne_bytes(bytes))
}

pub fn ip_addr_ntop(addr: IpAddr) -> String {
    let bytes = addr.to_ne_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn ip_print(data: &[u8]) {
    let Some(ip_hdr) = IpHdr::from_bytes(data) else {
        tracing::warn!("IP packet too short: len={}", data.len());
        return;
    };

    tracing::info!("IP Header: {}", ip_hdr);
    debugdump(data);
}

fn ip_input_handler(data: &[u8], dev: &Device, ctx: &ProtocolContexts) {
    if let Err(e) = ip_input(data, dev, ctx) {
        tracing::error!("ip_input error: {}", e);
    }
}

pub fn ip_input(data: &[u8], dev: &Device, _ctx: &ProtocolContexts) -> Result<()> {
    tracing::debug!("ip_input: dev={}, len={}", dev.name_string(), data.len());

    let hdr = IpHdr::from_bytes(data)
        .ok_or_else(|| anyhow::anyhow!("IP packet too short: len={}", data.len()))?;

    if hdr.version() != IP_VERSION_IPV4 {
        anyhow::bail!("Unsupported IP version: {}", hdr.version());
    }

    let hlen = hdr.hdr_len();
    if data.len() < hlen {
        anyhow::bail!(
            "IP packet too short for header length: len={}, hlen={}",
            data.len(),
            hlen
        );
    }

    if cksum16(&data[..hlen], 0) != 0 {
        anyhow::bail!("IP header checksum error");
    }

    let total = ntoh16(hdr.total) as usize;
    if data.len() < total {
        anyhow::bail!(
            "IP packet too short for total length: len={}, total={}",
            data.len(),
            total
        );
    }

    let offset = ntoh16(hdr.offset);
    if offset & (IP_HDR_FLAG_MF | IP_HDR_OFFSET_MASK) != 0 {
        anyhow::bail!("Fragmented IP packets are not supported");
    }

    let dst = hdr.dst;
    let matched = dev.ifaces.iter().any(|iface| match iface {
        NetIface::Ip(ip_iface) => ip_iface.is_destination_match(dst),
    });

    if !matched {
        tracing::debug!(
            "No matching IP interface found for dst={}",
            ip_addr_ntop(dst)
        );
        return Ok(());
    }

    tracing::debug!(
        "Packet accepted: src={}, dst={}, protocol={}",
        ip_addr_ntop(hdr.src),
        ip_addr_ntop(dst),
        hdr.protocol
    );

    ip_print(data);

    let payload = &data[hlen..total];
    ip_protocol_dispatch(hdr.protocol, payload, hdr.src, hdr.dst, dev, _ctx)?;

    Ok(())
}

fn ip_protocol_dispatch(
    protocol: u8,
    _payload: &[u8],
    _src: IpAddr,
    _dst: IpAddr,
    _dev: &Device,
    _ctx: &ProtocolContexts,
) -> Result<()> {
    match protocol {
        IP_PROTOCOL_ICMP => {
            tracing::debug!("Dispatching to ICMP (not yet implemented)");
        }
        IP_PROTOCOL_TCP => {
            tracing::debug!("Dispatching to TCP (not yet implemented)");
        }
        IP_PROTOCOL_UDP => {
            tracing::debug!("Dispatching to UDP (not yet implemented)");
        }
        _ => {
            tracing::debug!("Unknown IP protocol: {}", protocol);
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn ip_output(
    _protocol: u8,
    _payload: &[u8],
    _src: IpAddr,
    _dst: IpAddr,
    _dev: &Device,
    ctx: &ProtocolContexts,
) -> Result<()> {
    let _id = ctx.ip_id.next();

    // TODO: Build IP header, calculate checksum, send through device
    tracing::debug!("ip_output: not yet implemented");

    Ok(())
}

pub fn init(protocols: &mut ProtocolManager) -> Result<()> {
    protocols.register(ProtocolType::Ip, ip_input_handler)?;
    tracing::info!("IP protocol initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_addr_pton() {
        assert_eq!(ip_addr_pton("0.0.0.0").unwrap(), IpAddr::ANY);
        assert_eq!(ip_addr_pton("255.255.255.255").unwrap(), IpAddr::BROADCAST);
        assert_eq!(
            ip_addr_pton("127.0.0.1").unwrap(),
            IpAddr::from_ne_bytes([127, 0, 0, 1])
        );
        assert_eq!(
            ip_addr_pton("192.168.1.1").unwrap(),
            IpAddr::from_ne_bytes([192, 168, 1, 1])
        );

        assert!(ip_addr_pton("").is_err());
        assert!(ip_addr_pton("1.2.3").is_err());
        assert!(ip_addr_pton("1.2.3.4.5").is_err());
        assert!(ip_addr_pton("256.0.0.1").is_err());
        assert!(ip_addr_pton("a.b.c.d").is_err());
    }

    #[test]
    fn test_ip_addr_ntop() {
        assert_eq!(ip_addr_ntop(IpAddr::ANY), "0.0.0.0");
        assert_eq!(ip_addr_ntop(IpAddr::BROADCAST), "255.255.255.255");
        assert_eq!(
            ip_addr_ntop(IpAddr::from_ne_bytes([127, 0, 0, 1])),
            "127.0.0.1"
        );
        assert_eq!(
            ip_addr_ntop(IpAddr::from_ne_bytes([192, 168, 1, 1])),
            "192.168.1.1"
        );
    }

    #[test]
    fn test_ip_addr_roundtrip() {
        let addrs = ["0.0.0.0", "127.0.0.1", "192.168.1.1", "255.255.255.255"];
        for addr_str in addrs {
            let addr = ip_addr_pton(addr_str).unwrap();
            assert_eq!(ip_addr_ntop(addr), addr_str);
        }
    }
}
