use std::sync::atomic::{AtomicU16, Ordering};

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

#[derive(Default)]
pub struct ProtocolContexts {
    pub ip_id: IpIdManager,
}

impl ProtocolContexts {
    pub fn new() -> Self {
        Self::default()
    }
}
