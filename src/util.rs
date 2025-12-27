use std::io::Write;

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
