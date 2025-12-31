use std::fmt;

use crate::context::ProtocolContexts;
use crate::device::Device;
use crate::protocol::ip::IpAddr;
use crate::util::{cksum16, debugdump, ntoh16, ntoh32};

pub const ICMP_HDR_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IcmpType {
    EchoReply = 0,
    DestUnreachable = 3,
    SourceQuench = 4,
    Redirect = 5,
    Echo = 8,
    TimeExceeded = 11,
    ParameterProblem = 12,
    Timestamp = 13,
    TimestampReply = 14,
    InfoRequest = 15,
    InfoReply = 16,
}

impl IcmpType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IcmpType::EchoReply),
            3 => Some(IcmpType::DestUnreachable),
            4 => Some(IcmpType::SourceQuench),
            5 => Some(IcmpType::Redirect),
            8 => Some(IcmpType::Echo),
            11 => Some(IcmpType::TimeExceeded),
            12 => Some(IcmpType::ParameterProblem),
            13 => Some(IcmpType::Timestamp),
            14 => Some(IcmpType::TimestampReply),
            15 => Some(IcmpType::InfoRequest),
            16 => Some(IcmpType::InfoReply),
            _ => None,
        }
    }
}

/// ICMP Header (RFC 792)
///
/// Generic ICMP header format:
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     Type      |     Code      |          Checksum             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         Values (varies)                       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpHdr {
    pub type_: u8,
    pub code: u8,
    pub sum: u16,
    pub values: u32,
}

impl IcmpHdr {
    /// Parse ICMP header from byte slice
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < ICMP_HDR_SIZE {
            return None;
        }
        // Copy to avoid unaligned access issues with packed struct
        Some(Self {
            type_: data[0],
            code: data[1],
            sum: u16::from_be_bytes([data[2], data[3]]),
            values: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        })
    }

    /// Get the ICMP type as an enum
    pub fn type_enum(&self) -> Option<IcmpType> {
        IcmpType::from_u8(self.type_)
    }

    /// For Echo Request/Reply: extract identifier
    pub fn echo_id(&self) -> u16 {
        (self.values >> 16) as u16
    }

    /// For Echo Request/Reply: extract sequence number
    pub fn echo_seq(&self) -> u16 {
        (self.values & 0xFFFF) as u16
    }
}

impl fmt::Display for IcmpHdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy fields to avoid unaligned access issues with packed struct
        let sum = self.sum;
        let values = self.values;
        write!(
            f,
            "type={}, code={}, sum={:#06x}, values={:#010x}",
            self.type_, self.code, sum, values
        )
    }
}

/// Get ICMP type name string
fn icmp_type_ntoa(type_: u8) -> &'static str {
    match IcmpType::from_u8(type_) {
        Some(IcmpType::EchoReply) => "EchoReply",
        Some(IcmpType::DestUnreachable) => "DestinationUnreachable",
        Some(IcmpType::SourceQuench) => "SourceQuench",
        Some(IcmpType::Redirect) => "Redirect",
        Some(IcmpType::Echo) => "Echo",
        Some(IcmpType::TimeExceeded) => "TimeExceeded",
        Some(IcmpType::ParameterProblem) => "ParameterProblem",
        Some(IcmpType::Timestamp) => "Timestamp",
        Some(IcmpType::TimestampReply) => "TimestampReply",
        Some(IcmpType::InfoRequest) => "InformationRequest",
        Some(IcmpType::InfoReply) => "InformationReply",
        None => "Unknown",
    }
}

/// Print ICMP header information for debugging
fn icmp_print(data: &[u8]) {
    let Some(hdr) = IcmpHdr::from_bytes(data) else {
        return;
    };

    tracing::debug!("   type: {} ({})", hdr.type_, icmp_type_ntoa(hdr.type_));
    tracing::debug!("   code: {}", hdr.code);
    tracing::debug!("    sum: {:#06x}", ntoh16(hdr.sum));

    match hdr.type_enum() {
        Some(IcmpType::EchoReply) | Some(IcmpType::Echo) => {
            tracing::debug!("     id: {}", hdr.echo_id());
            tracing::debug!("    seq: {}", hdr.echo_seq());
        }
        Some(IcmpType::DestUnreachable) => {
            tracing::debug!(" unused: {}", ntoh32(hdr.values));
        }
        _ => {
            tracing::debug!("    dep: {:#010x}", ntoh32(hdr.values));
        }
    }

    debugdump(data);
}

pub fn input(data: &[u8], src: IpAddr, dst: IpAddr, _dev: &Device, _ctx: &ProtocolContexts) {
    // Validate minimum header size
    if data.len() < ICMP_HDR_SIZE {
        tracing::error!("icmp_input: too short, len={}", data.len());
        return;
    }

    // Verify checksum
    if cksum16(data, 0) != 0 {
        tracing::error!("icmp_input: checksum error");
        return;
    }

    tracing::debug!("{} => {}, len={}", src, dst, data.len());

    icmp_print(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icmp_hdr_from_bytes() {
        // ICMP Echo Request
        let icmp_data = [
            0x08, 0x00, 0x35, 0x64, // type=8 (Echo), code=0, checksum=0x3564
            0x00, 0x80, 0x00, 0x01, // id=128, seq=1
            0x31, 0x32, 0x33, 0x34, // payload: "1234..."
        ];

        let hdr = IcmpHdr::from_bytes(&icmp_data).unwrap();
        assert_eq!(hdr.type_, 8); // Echo Request
        assert_eq!(hdr.code, 0);
        assert_eq!(hdr.type_enum(), Some(IcmpType::Echo));
        assert_eq!(hdr.echo_id(), 128);
        assert_eq!(hdr.echo_seq(), 1);
    }

    #[test]
    fn test_icmp_hdr_too_short() {
        let short_data = [0x08, 0x00, 0x35]; // Only 3 bytes
        assert!(IcmpHdr::from_bytes(&short_data).is_none());
    }

    #[test]
    fn test_icmp_checksum() {
        // Valid ICMP packet with correct checksum (from TEST_ICMP_PACKET)
        let icmp_data = [
            0x08, 0x00, 0x35, 0x64, 0x00, 0x80, 0x00, 0x01, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
            0x37, 0x38, 0x39, 0x30, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5e, 0x26, 0x2a, 0x28, 0x29,
        ];
        assert_eq!(cksum16(&icmp_data, 0), 0);
    }

    #[test]
    fn test_icmp_type_conversion() {
        assert_eq!(IcmpType::from_u8(0), Some(IcmpType::EchoReply));
        assert_eq!(IcmpType::from_u8(8), Some(IcmpType::Echo));
        assert_eq!(IcmpType::from_u8(3), Some(IcmpType::DestUnreachable));
        assert_eq!(IcmpType::from_u8(255), None);
    }

    #[test]
    fn test_icmp_type_ntoa() {
        assert_eq!(icmp_type_ntoa(0), "EchoReply");
        assert_eq!(icmp_type_ntoa(3), "DestinationUnreachable");
        assert_eq!(icmp_type_ntoa(8), "Echo");
        assert_eq!(icmp_type_ntoa(11), "TimeExceeded");
        assert_eq!(icmp_type_ntoa(255), "Unknown");
    }
}
