use std::fmt;

use anyhow::Result;

use crate::{
    net::{self, Device},
    util::{cksum16, debugdump, ntoh16},
};

// IP version
pub const IP_VERSION_IPV4: u8 = 4;

// IP header size
pub const IP_HDR_SIZE_MIN: usize = 20;
pub const IP_HDR_SIZE_MAX: usize = 60;

// IP packet size
pub const IP_TOTAL_SIZE_MAX: usize = u16::MAX as usize;
pub const IP_PAYLOAD_SIZE_MAX: usize = IP_TOTAL_SIZE_MAX - IP_HDR_SIZE_MIN;

// IP address
pub const IP_ADDR_LEN: usize = 4;
pub const IP_ADDR_STR_LEN: usize = 16; // "ddd.ddd.ddd.ddd\0"

// IP header flags
const IP_HDR_FLAG_MF: u16 = 0x2000; // more fragments flag
#[allow(dead_code)]
const IP_HDR_FLAG_DF: u16 = 0x4000; // don't fragment flag
#[allow(dead_code)]
const IP_HDR_FLAG_RF: u16 = 0x8000; // reserved flag
const IP_HDR_OFFSET_MASK: u16 = 0x1fff;

/// IP address type (network byte order)
pub type IpAddr = u32;

/// Special IP addresses
pub const IP_ADDR_ANY: IpAddr = 0x00000000; // 0.0.0.0
pub const IP_ADDR_BROADCAST: IpAddr = 0xffffffff; // 255.255.255.255

/// IP header structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IpHdr {
    pub vhl: u8,      // version (4 bits) + header length (4 bits)
    pub tos: u8,      // type of service
    pub total: u16,   // total length
    pub id: u16,      // identification
    pub offset: u16,  // flags (3 bits) + fragment offset (13 bits)
    pub ttl: u8,      // time to live
    pub protocol: u8, // protocol
    pub sum: u16,     // header checksum
    pub src: IpAddr,  // source address
    pub dst: IpAddr,  // destination address
}

impl IpHdr {
    pub fn new(data: &[u8]) -> &Self {
        unsafe { &*(data.as_ptr() as *const IpHdr) }
    }

    /// Get IP version from vhl field
    pub fn version(&self) -> u8 {
        (self.vhl >> 4) & 0x0f
    }

    /// Get header length in bytes from vhl field
    pub fn hdr_len(&self) -> usize {
        ((self.vhl & 0x0f) as usize) * 4
    }
}

/// Convert IP address from presentation format (string) to network format
/// e.g., "192.168.1.1" -> network byte order u32
/// The resulting value stores bytes in network byte order in memory.
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

    Ok(u32::from_ne_bytes(bytes))
}

/// Convert IP address from network format to presentation format (string)
/// The address is stored in network byte order (big-endian) in memory,
/// so we use to_ne_bytes() to get the raw bytes as they appear in memory.
pub fn ip_addr_ntop(addr: IpAddr) -> String {
    let bytes = addr.to_ne_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
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

fn ip_print(data: &[u8]) {
    if data.len() < IP_HDR_SIZE_MIN {
        tracing::warn!("IP packet too short: len={}", data.len());
        return;
    }

    let ip_hdr = IpHdr::new(data);
    tracing::info!("IP Header: {}", ip_hdr);
    debugdump(data);
}

fn ip_input_handler(data: &[u8], dev: &Device) {
    if let Err(e) = ip_input(data, dev) {
        tracing::error!("ip_input error: {}", e);
    }
}

fn ip_input(data: &[u8], dev: &Device) -> Result<()> {
    tracing::debug!("dev={}, len={}", dev.name_string(), data.len());

    if data.len() < IP_HDR_SIZE_MIN {
        anyhow::bail!("IP packet too short: len={}", data.len());
    }
    let hdr = IpHdr::new(data);
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

    // Verify checksum
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

    ip_print(data);

    Ok(())
}

pub fn init(net_stack: &mut net::NetStack) -> Result<()> {
    net_stack.register_protocol(net::NET_PROTOCOL_TYPE_IP, ip_input_handler)?;
    tracing::info!("initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_addr_pton() {
        // Test valid addresses
        assert_eq!(ip_addr_pton("0.0.0.0").unwrap(), IP_ADDR_ANY);
        assert_eq!(ip_addr_pton("255.255.255.255").unwrap(), IP_ADDR_BROADCAST);
        assert_eq!(
            ip_addr_pton("127.0.0.1").unwrap(),
            u32::from_ne_bytes([127, 0, 0, 1])
        );
        assert_eq!(
            ip_addr_pton("192.168.1.1").unwrap(),
            u32::from_ne_bytes([192, 168, 1, 1])
        );

        // Test invalid addresses
        assert!(ip_addr_pton("").is_err());
        assert!(ip_addr_pton("1.2.3").is_err());
        assert!(ip_addr_pton("1.2.3.4.5").is_err());
        assert!(ip_addr_pton("256.0.0.1").is_err());
        assert!(ip_addr_pton("a.b.c.d").is_err());
    }

    #[test]
    fn test_ip_addr_ntop() {
        assert_eq!(ip_addr_ntop(IP_ADDR_ANY), "0.0.0.0");
        assert_eq!(ip_addr_ntop(IP_ADDR_BROADCAST), "255.255.255.255");
        assert_eq!(
            ip_addr_ntop(u32::from_ne_bytes([127, 0, 0, 1])),
            "127.0.0.1"
        );
        assert_eq!(
            ip_addr_ntop(u32::from_ne_bytes([192, 168, 1, 1])),
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
