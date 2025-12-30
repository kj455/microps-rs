use std::fmt;
use std::fmt::Display;
use std::ops::{BitAnd, BitOr, Not};

use anyhow::Result;

use super::{PROTOCOL_TYPE_IP, ProtocolManager, ProtocolType};
use crate::context::ProtocolContexts;
use crate::device::{Device, DeviceManager, NET_DEVICE_FLAG_NEED_ARP};
use crate::iface::{IpIface, NetIface};
use crate::protocol::icmp;
use crate::util::{cksum16, debugdump, hton16, ntoh16};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpProtocol {
    Icmp,
    Tcp,
    Udp,
    Other(u8),
}

impl IpProtocol {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => IpProtocol::Icmp,
            6 => IpProtocol::Tcp,
            17 => IpProtocol::Udp,
            other => IpProtocol::Other(other),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            IpProtocol::Icmp => 1,
            IpProtocol::Tcp => 6,
            IpProtocol::Udp => 17,
            IpProtocol::Other(v) => v,
        }
    }
}

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

    pub fn from_str(s: &str) -> Result<Self> {
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

    pub fn to_string(self) -> String {
        let bytes = self.to_ne_bytes();
        format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

impl Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.to_ne_bytes();
        write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
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
    pub fn new(
        protocol: IpProtocol,
        total: u16,
        id: u16,
        offset: u16,
        src: IpAddr,
        dst: IpAddr,
    ) -> Self {
        let hlen = IP_HDR_SIZE_MIN;
        let vhl = (IP_VERSION_IPV4 << 4) | ((hlen / 4) as u8);
        Self {
            vhl,
            tos: 0,
            total: hton16(total),
            id: hton16(id),
            offset: hton16(offset),
            ttl: IP_TTL_DEFAULT,
            protocol: protocol.to_u8(),
            sum: 0,
            src,
            dst,
        }
    }

    pub fn to_bytes(&self) -> [u8; IP_HDR_SIZE_MIN] {
        // SAFETY: IpHdr is #[repr(C, packed)] and exactly IP_HDR_SIZE_MIN bytes
        unsafe { std::mem::transmute_copy(self) }
    }

    pub fn with_checksum(mut self) -> Self {
        self.sum = 0;
        let bytes = self.to_bytes();
        self.sum = hton16(cksum16(&bytes, 0));
        self
    }

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

    pub fn protocol(&self) -> IpProtocol {
        IpProtocol::from_u8(self.protocol)
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
            self.src.to_string(),
            self.dst.to_string()
        )
    }
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
        tracing::debug!("No matching IP interface found for dst={}", dst.to_string());
        return Ok(());
    }

    tracing::debug!(
        "Packet accepted: src={}, dst={}, protocol={:?}",
        hdr.src.to_string(),
        hdr.dst.to_string(),
        hdr.protocol()
    );

    ip_print(data);

    let payload = &data[hlen..total];
    match hdr.protocol() {
        IpProtocol::Icmp => {
            icmp::input(payload, hdr.src, hdr.dst, dev, _ctx);
        }
        IpProtocol::Tcp => {
            tracing::debug!("Dispatching to TCP (not yet implemented)");
        }
        IpProtocol::Udp => {
            tracing::debug!("Dispatching to UDP (not yet implemented)");
        }
        IpProtocol::Other(p) => {
            tracing::debug!("Unknown IP protocol: {}", p);
        }
    }

    Ok(())
}

const IP_TTL_DEFAULT: u8 = 0xff;

/// Generate a random 16-bit ID for IP packets
fn random16() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u16;
    seed.wrapping_add(std::process::id() as u16)
}

/// Register an IP interface on a device and global registry (single API).
/// Equivalent to C's ip_iface_register.
pub fn register_iface(
    dev: &mut Device,
    unicast: &str,
    netmask: &str,
    ctx: &mut ProtocolContexts,
) -> Result<()> {
    let iface = IpIface::new(unicast, netmask, dev.index)?;

    tracing::info!(
        "dev={}, unicast={}, netmask={}, broadcast={}",
        dev.name_string(),
        unicast,
        netmask,
        iface.broadcast,
    );

    // 1. Register on device
    dev.ifaces.push(NetIface::Ip(iface.clone()));

    // 2. Register in global registry
    ctx.ip_ifaces.register(iface)?;

    Ok(())
}

/// Output IP packet to the device associated with the given interface.
fn output_device(
    iface: &IpIface,
    data: &[u8],
    target: IpAddr,
    devices: &DeviceManager,
) -> Result<()> {
    tracing::debug!(
        "ip_output_device: dev={}, len={}, target={}",
        iface.device_index,
        data.len(),
        target.to_string()
    );

    let dev = devices
        .get(iface.device_index)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", iface.device_index))?;

    let hwaddr: Option<&[u8]> = if dev.flags & NET_DEVICE_FLAG_NEED_ARP != 0 {
        if target == iface.broadcast || target == IpAddr::BROADCAST {
            Some(&dev.broadcast[..dev.alen as usize])
        } else {
            anyhow::bail!("ARP does not implement");
        }
    } else {
        None
    };

    dev.output(PROTOCOL_TYPE_IP, data, hwaddr)
}

/// Build an IP packet with header and payload.
fn build_packet(
    protocol: IpProtocol,
    data: &[u8],
    id: u16,
    offset: u16,
    src: IpAddr,
    dst: IpAddr,
    buf: &mut [u8],
) -> Result<usize> {
    let hlen = IP_HDR_SIZE_MIN;
    let total = hlen + data.len();

    if buf.len() < total {
        anyhow::bail!("Buffer too small: need {}, have {}", total, buf.len());
    }

    let hdr = IpHdr::new(protocol, total as u16, id, offset, src, dst).with_checksum();

    buf[..hlen].copy_from_slice(&hdr.to_bytes());
    buf[hlen..total].copy_from_slice(data);

    ip_print(&buf[..total]);

    Ok(total)
}

/// Send an IP packet with the given payload.
pub fn ip_output(
    protocol: IpProtocol,
    payload: &[u8],
    src: IpAddr,
    dst: IpAddr,
    ctx: &ProtocolContexts,
    devices: &DeviceManager,
) -> Result<isize> {
    tracing::debug!(
        "ip_output: {} => {}, protocol={:?}, len={}",
        src.to_string(),
        dst.to_string(),
        protocol,
        payload.len()
    );

    // Routing not implemented - require explicit source address
    if src == IpAddr::ANY {
        anyhow::bail!("ip routing does not implement");
    }

    // Find interface by source address
    let iface = ctx
        .ip_ifaces
        .select(src)
        .ok_or_else(|| anyhow::anyhow!("iface not found, src={}", src.to_string()))?;

    // Check if destination is reachable (same network or broadcast)
    let src_network = iface.unicast & iface.netmask;
    let dst_network = dst & iface.netmask;
    if dst_network != src_network && dst != IpAddr::BROADCAST {
        anyhow::bail!("not reached, dst={}", dst.to_string());
    }

    // Check MTU
    let dev = devices
        .get(iface.device_index)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", iface.device_index))?;

    if (dev.mtu as usize) < IP_HDR_SIZE_MIN + payload.len() {
        anyhow::bail!(
            "too long, dev={}, mtu={} < {}",
            dev.name_string(),
            dev.mtu,
            IP_HDR_SIZE_MIN + payload.len()
        );
    }

    // Build packet
    let id = random16();
    let mut buf = [0u8; IP_TOTAL_SIZE_MAX];
    let packet_len = build_packet(protocol, payload, id, 0, iface.unicast, dst, &mut buf)?;

    // Send packet
    output_device(iface, &buf[..packet_len], dst, devices)?;

    Ok(packet_len as isize)
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
    fn test_ip_addr_from_str() {
        assert_eq!(IpAddr::from_str("0.0.0.0").unwrap(), IpAddr::ANY);
        assert_eq!(
            IpAddr::from_str("255.255.255.255").unwrap(),
            IpAddr::BROADCAST
        );
        assert_eq!(
            IpAddr::from_str("127.0.0.1").unwrap(),
            IpAddr::from_ne_bytes([127, 0, 0, 1])
        );
        assert_eq!(
            IpAddr::from_str("192.168.1.1").unwrap(),
            IpAddr::from_ne_bytes([192, 168, 1, 1])
        );

        assert!(IpAddr::from_str("").is_err());
        assert!(IpAddr::from_str("1.2.3").is_err());
        assert!(IpAddr::from_str("1.2.3.4.5").is_err());
        assert!(IpAddr::from_str("256.0.0.1").is_err());
        assert!(IpAddr::from_str("a.b.c.d").is_err());
    }

    #[test]
    fn test_ip_addr_to_string() {
        assert_eq!(IpAddr::to_string(IpAddr::ANY), "0.0.0.0");
        assert_eq!(IpAddr::to_string(IpAddr::BROADCAST), "255.255.255.255");
        assert_eq!(
            IpAddr::to_string(IpAddr::from_ne_bytes([127, 0, 0, 1])),
            "127.0.0.1"
        );
        assert_eq!(
            IpAddr::to_string(IpAddr::from_ne_bytes([192, 168, 1, 1])),
            "192.168.1.1"
        );
    }

    #[test]
    fn test_ip_addr_roundtrip() {
        let addrs = ["0.0.0.0", "127.0.0.1", "192.168.1.1", "255.255.255.255"];
        for addr_str in addrs {
            let addr = IpAddr::from_str(addr_str).unwrap();
            assert_eq!(IpAddr::to_string(addr), addr_str);
        }
    }
}
