use anyhow::Result;
use std::rc::Rc;

use super::{Device, DeviceIndex, DeviceManager, DeviceOps, DeviceType, NET_DEVICE_FLAG_LOOPBACK};
use crate::util::debugdump;

const LOOPBACK_MTU: u16 = u16::MAX;

// Will be replaced with IRQ-based signaling in the future
pub type OutputCallback = Rc<dyn Fn(u16, &[u8], DeviceIndex)>;

struct LoopbackOps {
    output_callback: OutputCallback,
}

impl DeviceOps for LoopbackOps {
    fn open(&self, _dev: &Device) -> Result<()> {
        Ok(())
    }

    fn close(&self, _dev: &Device) -> Result<()> {
        Ok(())
    }

    fn transmit(&self, dev: &Device, type_: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()> {
        tracing::debug!(
            "loopback_transmit: type=0x{:04x}, len={}, dst={:?}",
            type_,
            data.len(),
            dst
        );
        debugdump(data);

        // HACK: Will be replaced with IRQ-based signaling in the future
        (self.output_callback)(type_, data, dev.index);

        Ok(())
    }
}

pub fn init(devices: &mut DeviceManager, output_callback: OutputCallback) -> Result<DeviceIndex> {
    let dev = Device {
        device_type: DeviceType::Loopback,
        mtu: LOOPBACK_MTU,
        flags: NET_DEVICE_FLAG_LOOPBACK,
        // Set after registration to avoid circular dependency
        ops: None,
        ..Default::default()
    };

    let index = devices.register(dev)?;

    if let Some(dev) = devices.get_mut(index) {
        dev.ops = Some(Box::new(LoopbackOps { output_callback }));
        tracing::info!("Loopback device initialized: {}", dev.name_string());
    }

    Ok(index)
}
