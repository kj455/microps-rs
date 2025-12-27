use anyhow::Result;
use std::rc::Rc;

use crate::{
    net::{
        Device, DeviceDescriptor, DeviceOps, NET_DEVICE_FLAG_LOOPBACK, NET_DEVICE_TYPE_LOOPBACK,
        NetStack,
    },
    util::debugdump,
};

/// Maximum size of IP datagram
const LOOPBACK_MTU: u16 = u16::MAX;

/// Input callback type for protocol dispatching
pub type OutputCallback = Rc<dyn Fn(u16, &[u8], DeviceDescriptor)>;

/// Loopback device operations
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

    fn output(&self, dev: &Device, type_: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()> {
        tracing::debug!(
            "loopback_output: type=0x{:04x}, len={}, dst={:?}",
            type_,
            data.len(),
            dst
        );
        debugdump(data);

        // HACK: Call the input callback for protocol dispatching
        (self.output_callback)(type_, data, dev.descriptor);

        Ok(())
    }
}

/// Initialize loopback device
pub fn init(net_stack: &mut NetStack, output_callback: OutputCallback) -> Result<DeviceDescriptor> {
    let dev = Device {
        device_type: NET_DEVICE_TYPE_LOOPBACK,
        mtu: LOOPBACK_MTU,
        flags: NET_DEVICE_FLAG_LOOPBACK,
        ops: None, // We'll set this after registration to avoid circular dependency
        ..Default::default()
    };

    let descriptor = net_stack.register_device(dev)?;

    // Now set the ops with the device descriptor and input callback
    if let Some(dev) = net_stack.get_device_mut(descriptor) {
        dev.ops = Some(Box::new(LoopbackOps { output_callback }));
        tracing::info!("success, dev={}", dev.name_string());
    }

    Ok(descriptor)
}
