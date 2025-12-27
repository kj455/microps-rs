use std::io::Write;

/// Convert 16-bit value from network byte order to host byte order
#[inline]
pub fn ntoh16(n: u16) -> u16 {
    u16::from_be(n)
}

/// Convert 16-bit value from host byte order to network byte order
#[inline]
pub fn hton16(h: u16) -> u16 {
    h.to_be()
}

/// Convert 32-bit value from network byte order to host byte order
#[inline]
pub fn ntoh32(n: u32) -> u32 {
    u32::from_be(n)
}

/// Convert 32-bit value from host byte order to network byte order
#[inline]
pub fn hton32(h: u32) -> u32 {
    h.to_be()
}

/// Internet checksum (RFC 1071)
/// Computes 16-bit one's complement sum
///
/// # Arguments
/// * `data` - byte slice to checksum
/// * `init` - initial value (used for pseudo-header in TCP/UDP)
///
/// # Returns
/// The one's complement of the one's complement sum
pub fn cksum16(data: &[u8], init: u32) -> u16 {
    let mut sum = init;

    // Process 16-bit words
    let mut chunks = data.chunks_exact(2);
    for chunk in chunks.by_ref() {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }

    // Handle odd byte
    if let Some(&last) = chunks.remainder().first() {
        sum += (last as u32) << 8;
    }

    // Fold 32-bit sum to 16 bits
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

/// Hexdump utility for debugging
/// Outputs data in hexadecimal and ASCII format
fn hexdump(data: &[u8]) {
    let mut stderr = std::io::stderr();
    let _ = writeln!(
        stderr,
        "+------+-------------------------------------------------+------------------+"
    );

    for offset in (0..data.len()).step_by(16) {
        // Print offset
        let _ = write!(stderr, "| {:04x} | ", offset);

        // Print hex values
        for index in 0..16 {
            if offset + index < data.len() {
                let _ = write!(stderr, "{:02x} ", data[offset + index]);
            } else {
                let _ = write!(stderr, "   ");
            }
        }

        let _ = write!(stderr, "| ");

        // Print ASCII representation
        for index in 0..16 {
            if offset + index < data.len() {
                let byte = data[offset + index];
                if byte.is_ascii() && !byte.is_ascii_control() {
                    let _ = write!(stderr, "{}", byte as char);
                } else {
                    let _ = write!(stderr, ".");
                }
            } else {
                let _ = write!(stderr, " ");
            }
        }

        let _ = writeln!(stderr, " |");
    }

    let _ = writeln!(
        stderr,
        "+------+-------------------------------------------------+------------------+"
    );
}

/// Conditional debug dump - only active in debug builds
#[cfg(debug_assertions)]
pub fn debugdump(data: &[u8]) {
    hexdump(data);
}

#[cfg(not(debug_assertions))]
pub fn debugdump(_data: &[u8]) {
    // No-op in release builds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cksum16_valid_ip_header() {
        // Valid IP header with correct checksum
        // This is the ICMP packet from main.rs
        let ip_header = [
            0x45, 0x00, 0x00, 0x30, // vhl, tos, total length
            0x00, 0x80, 0x00, 0x00, // id, flags/offset
            0xff, 0x01, 0xbd, 0x4a, // ttl, protocol, checksum
            0x7f, 0x00, 0x00, 0x01, // src: 127.0.0.1
            0x7f, 0x00, 0x00, 0x01, // dst: 127.0.0.1
        ];
        // Valid checksum should result in 0
        assert_eq!(cksum16(&ip_header, 0), 0);
    }

    #[test]
    fn test_cksum16_compute() {
        // IP header without checksum (checksum field = 0)
        let mut ip_header = [
            0x45, 0x00, 0x00, 0x30, // vhl, tos, total length
            0x00, 0x80, 0x00, 0x00, // id, flags/offset
            0xff, 0x01, 0x00, 0x00, // ttl, protocol, checksum (0)
            0x7f, 0x00, 0x00, 0x01, // src: 127.0.0.1
            0x7f, 0x00, 0x00, 0x01, // dst: 127.0.0.1
        ];

        // Compute checksum
        let checksum = cksum16(&ip_header, 0);
        ip_header[10] = (checksum >> 8) as u8;
        ip_header[11] = (checksum & 0xff) as u8;

        // Verify: checksum of header with correct checksum should be 0
        assert_eq!(cksum16(&ip_header, 0), 0);
    }

    #[test]
    fn test_cksum16_odd_length() {
        // Test with odd number of bytes
        let data = [0x01, 0x02, 0x03];
        let _ = cksum16(&data, 0); // Should not panic
    }
}
