use anyhow::{Context, Result};
use std::sync::Arc;

use crate::util::debugdump;

// Constants
const IFNAMSIZ: usize = 16;

/// Device descriptor (index into the devices array)
pub type DeviceDescriptor = usize;

/// Device operations trait
/// This trait defines the interface for network device drivers
pub trait DeviceOps: Send + Sync {
    fn open(&self, dev: &mut Device) -> Result<()>;
    fn close(&self, dev: &mut Device) -> Result<()>;
    fn output(&self, dev: &Device, type_: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()>;
}

// Device Types
pub const NET_DEVICE_TYPE_DUMMY: u16 = 0x0000;
pub const NET_DEVICE_TYPE_LOOPBACK: u16 = 0x0001;
pub const NET_DEVICE_TYPE_ETHERNET: u16 = 0x0002;

// Device Flags
pub const NET_DEVICE_FLAG_UP: u16 = 0x0001;
pub const NET_DEVICE_FLAG_LOOPBACK: u16 = 0x0010;
pub const NET_DEVICE_FLAG_BROADCAST: u16 = 0x0020;
pub const NET_DEVICE_FLAG_P2P: u16 = 0x0040;
pub const NET_DEVICE_FLAG_NEED_ARP: u16 = 0x0100;

const NET_DEVICE_ADDR_LEN: usize = 16;

/// Network device structure
#[allow(dead_code)]
pub struct Device {
    pub index: u32,
    pub name: [u8; IFNAMSIZ],
    pub device_type: u16,
    pub mtu: u16,
    pub flags: u16,
    pub hlen: u16,
    pub alen: u16,
    pub addr: [u8; NET_DEVICE_ADDR_LEN],
    pub broadcast: [u8; NET_DEVICE_ADDR_LEN],
    pub ops: Option<Arc<dyn DeviceOps>>,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            index: 0,
            name: [0; IFNAMSIZ],
            device_type: 0,
            mtu: 0,
            flags: 0,
            hlen: 0,
            alen: 0,
            addr: [0; NET_DEVICE_ADDR_LEN],
            broadcast: [0; NET_DEVICE_ADDR_LEN],
            ops: None,
        }
    }
}

impl Device {
    /// Check if the device is UP
    pub fn is_up(&self) -> bool {
        (self.flags & NET_DEVICE_FLAG_UP) != 0
    }

    /// Get device state as string
    pub fn state(&self) -> &str {
        if self.is_up() { "UP" } else { "DOWN" }
    }

    pub fn name_string(&self) -> String {
        String::from_utf8_lossy(&self.name).to_string()
    }

    /// Output data to this device
    pub fn output(&self, device_type: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()> {
        let dev_name = self.name_string();
        tracing::debug!(
            "dev={}, type=0x{:04x}, len={}",
            dev_name,
            device_type,
            data.len()
        );
        debugdump(data);

        if !self.is_up() {
            anyhow::bail!("device not opened");
        }
        if data.len() > self.mtu as usize {
            anyhow::bail!("data too long");
        }

        // Clone the Arc to avoid borrowing issues
        if let Some(ops) = &self.ops {
            ops.output(self, device_type, data, dst)?;
        }

        Ok(())
    }

    /// Process input data from this device
    pub fn input(&self, type_: u16, data: &[u8]) -> Result<()> {
        tracing::debug!(
            "dev={}, type=0x{:04x}, len={}",
            self.name_string(),
            type_,
            data.len()
        );
        debugdump(data);
        // TODO: Pass to protocol stack for processing
        Ok(())
    }

    /// Open this device
    pub fn open(&mut self) -> Result<()> {
        let dev_name = self.name_string();
        tracing::info!("Opening device: {}", dev_name);

        if self.is_up() {
            anyhow::bail!("device already opened");
        }

        // Clone the Arc to avoid borrowing issues
        if let Some(ops) = self.ops.clone() {
            ops.open(self)?;
        }

        self.flags |= NET_DEVICE_FLAG_UP;
        Ok(())
    }

    /// Close this device
    pub fn close(&mut self) -> Result<()> {
        let dev_name = self.name_string();
        tracing::info!("Closing device: {}", dev_name);

        if !self.is_up() {
            anyhow::bail!("device not opened");
        }

        // Clone the Arc to avoid borrowing issues
        if let Some(ops) = self.ops.clone() {
            ops.close(self)?;
        }

        self.flags &= !NET_DEVICE_FLAG_UP;
        Ok(())
    }
}

/// Network devices manager
pub struct NetDevices {
    devices: Vec<Device>,
}

impl NetDevices {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Register a network device
    /// Returns the device descriptor
    pub fn register(&mut self, mut dev: Device) -> Result<DeviceDescriptor> {
        dev.index = self.devices.len() as u32;

        // Generate device name (net0, net1, etc.)
        let name_str = format!("net{}", dev.index);
        let name_bytes = name_str.as_bytes();
        dev.name[..name_bytes.len()].copy_from_slice(name_bytes);

        tracing::info!("success, dev={}, type=0x{:04x}", name_str, dev.device_type);

        let descriptor = self.devices.len();
        self.devices.push(dev);

        Ok(descriptor)
    }

    /// Get a reference to a device by descriptor
    pub fn get(&self, descriptor: DeviceDescriptor) -> Option<&Device> {
        self.devices.get(descriptor)
    }

    /// Get a mutable reference to a device by descriptor
    pub fn get_mut(&mut self, descriptor: DeviceDescriptor) -> Option<&mut Device> {
        self.devices.get_mut(descriptor)
    }

    /// Iterate over all devices
    pub fn iter(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter()
    }

    /// Iterate over all devices mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Device> {
        self.devices.iter_mut()
    }

    /// Initialize the network stack
    pub fn init(&mut self) -> Result<()> {
        tracing::info!("setup protocol stack...");
        // TODO: Implement actual network initialization
        Ok(())
    }

    /// Start all network devices
    pub fn run(&mut self) -> Result<()> {
        tracing::info!("startup...");

        for dev in self.iter_mut() {
            let dev_name = dev.name_string();
            dev.open()
                .with_context(|| format!("Failed to open device: {}", dev_name))?;
        }

        tracing::info!("success");
        Ok(())
    }

    /// Shutdown all network devices
    pub fn shutdown(&mut self) -> Result<()> {
        tracing::info!("shutting down...");

        for dev in self.iter_mut() {
            let dev_name = dev.name_string();
            dev.close()
                .with_context(|| format!("Failed to close device: {}", dev_name))?;
        }

        tracing::info!("success");
        Ok(())
    }
}
