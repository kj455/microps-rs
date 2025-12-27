use anyhow::Result;
use std::sync::Arc;

use crate::{
    net::{
        Device, DeviceDescriptor, DeviceOps, NET_DEVICE_FLAG_LOOPBACK, NET_DEVICE_TYPE_LOOPBACK,
        NetDevices,
    },
    util::debugdump,
};

/// Maximum size of IP datagram
const LOOPBACK_MTU: u16 = u16::MAX;

/// Loopback device operations
struct LoopbackOps {}

impl DeviceOps for LoopbackOps {
    fn open(&self, _dev: &mut Device) -> Result<()> {
        Ok(())
    }

    fn close(&self, _dev: &mut Device) -> Result<()> {
        Ok(())
    }

    fn output(&self, dev: &Device, type_: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()> {
        tracing::debug!(
            "loopback_output: type=0x{:04x}, len={}, dst={:?}",
            type_,
            data.len(),
            dst
        );
        debugdump(data);
        dev.input(type_, data)
    }
}

/// Initialize loopback device
pub fn init(devices: &mut NetDevices) -> Result<DeviceDescriptor> {
    let dev = Device {
        device_type: NET_DEVICE_TYPE_LOOPBACK,
        mtu: LOOPBACK_MTU,
        flags: NET_DEVICE_FLAG_LOOPBACK,
        ops: None, // We'll set this after registration to avoid circular dependency
        ..Default::default()
    };

    let descriptor = devices.register(dev)?;

    // Now set the ops with the device descriptor
    if let Some(dev) = devices.get_mut(descriptor) {
        let ops = Arc::new(LoopbackOps {});
        dev.ops = Some(ops);
        tracing::info!("success, dev={}", dev.name_string());
    }

    Ok(descriptor)
}
