use anyhow::{Context, Result};

use crate::{ip, util::debugdump};

// Constants
const IFNAMSIZ: usize = 16;

/// Device descriptor (index into the devices array)
pub type DeviceDescriptor = usize;

/// Protocol handler function type
pub type ProtocolHandler = fn(&[u8], &Device);

/// Network protocol structure
struct Protocol {
    type_: u16,
    handler: ProtocolHandler,
}

/// Device operations trait
/// This trait defines the interface for network device drivers
pub trait DeviceOps {
    fn open(&self, dev: &Device) -> Result<()>;
    fn close(&self, dev: &Device) -> Result<()>;
    fn output(&self, dev: &Device, type_: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()>;
}

// Device Types
pub const NET_DEVICE_TYPE_DUMMY: u16 = 0x0000;
pub const NET_DEVICE_TYPE_LOOPBACK: u16 = 0x0001;
pub const NET_DEVICE_TYPE_ETHERNET: u16 = 0x0002;

// Protocol Types (use same values as Ethernet types)
pub const NET_PROTOCOL_TYPE_IP: u16 = 0x0800;
pub const NET_PROTOCOL_TYPE_ARP: u16 = 0x0806;
pub const NET_PROTOCOL_TYPE_IPV6: u16 = 0x86dd;

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
    pub descriptor: DeviceDescriptor,
    pub name: [u8; IFNAMSIZ],
    pub device_type: u16,
    pub mtu: u16,
    pub flags: u16,
    pub hlen: u16,
    pub alen: u16,
    pub addr: [u8; NET_DEVICE_ADDR_LEN],
    pub broadcast: [u8; NET_DEVICE_ADDR_LEN],
    pub ops: Option<Box<dyn DeviceOps>>,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            descriptor: DeviceDescriptor::default(),
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
            "device_output: dev={}, type=0x{:04x}, len={}",
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
    /// NOTE: For proper protocol dispatch, use NetDevices::input() instead
    pub fn input(&self, type_: u16, data: &[u8]) -> Result<()> {
        tracing::debug!(
            "device_input: dev={}, type=0x{:04x}, len={}",
            self.name_string(),
            type_,
            data.len()
        );
        debugdump(data);
        // TODO: This should dispatch to NetDevices::input() for protocol handling
        // Currently, loopback uses this method directly, which needs architectural changes
        Ok(())
    }

    /// Open this device
    pub fn open(&mut self) -> Result<()> {
        let dev_name = self.name_string();
        tracing::info!("Opening device: {}", dev_name);

        if self.is_up() {
            anyhow::bail!("device already opened");
        }

        if let Some(ops) = &self.ops {
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

        if let Some(ops) = &self.ops {
            ops.close(self)?;
        }

        self.flags &= !NET_DEVICE_FLAG_UP;
        Ok(())
    }
}

pub struct NetStack {
    devices: Vec<Device>,
    protocols: Vec<Protocol>,
}

impl NetStack {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            protocols: Vec::new(),
        }
    }

    /// Register a network device
    /// Returns the device descriptor
    pub fn register_device(&mut self, mut dev: Device) -> Result<DeviceDescriptor> {
        let descriptor = self.devices.len();
        dev.descriptor = descriptor;

        // Generate device name (net0, net1, etc.)
        let name_str = format!("net{}", dev.descriptor);
        let name_bytes = name_str.as_bytes();
        dev.name[..name_bytes.len()].copy_from_slice(name_bytes);

        tracing::info!("success, dev={}, type=0x{:04x}", name_str, dev.device_type);

        self.devices.push(dev);

        Ok(descriptor)
    }

    /// Get a reference to a device by descriptor
    pub fn get_device(&self, descriptor: DeviceDescriptor) -> Option<&Device> {
        self.devices.get(descriptor)
    }

    /// Get a mutable reference to a device by descriptor
    pub fn get_device_mut(&mut self, descriptor: DeviceDescriptor) -> Option<&mut Device> {
        self.devices.get_mut(descriptor)
    }

    /// Iterate over all devices
    pub fn iter_devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter()
    }

    /// Iterate over all devices mutably
    pub fn iter_mut_devices(&mut self) -> impl Iterator<Item = &mut Device> {
        self.devices.iter_mut()
    }

    /// Register a protocol handler
    /// NOTE: must not be called after net_run()
    pub fn register_protocol(&mut self, type_: u16, handler: ProtocolHandler) -> Result<()> {
        tracing::debug!("registering protocol: type=0x{:04x}", type_);

        // Check if protocol is already registered
        if self.protocols.iter().any(|p| p.type_ == type_) {
            anyhow::bail!("Protocol already registered: type=0x{:04x}", type_);
        }

        self.protocols.push(Protocol { type_, handler });
        Ok(())
    }

    /// Process input data from a device and dispatch to protocol handler
    pub fn input(&self, type_: u16, data: &[u8], descriptor: DeviceDescriptor) -> Result<()> {
        let dev = self
            .get_device(descriptor)
            .ok_or_else(|| anyhow::anyhow!("Invalid device descriptor: {}", descriptor))?;

        tracing::debug!(
            "net_input: dev={}, type=0x{:04x}, len={}",
            dev.name_string(),
            type_,
            data.len()
        );

        // Find and call the appropriate protocol handler
        for protocol in &self.protocols {
            if protocol.type_ == type_ {
                (protocol.handler)(data, dev);
                return Ok(());
            }
        }

        tracing::debug!("no protocol handler registered for type: 0x{:04x}", type_);
        Ok(())
    }

    pub fn init(&mut self) -> Result<()> {
        tracing::info!("setup protocol stack...");

        ip::init(self)?;

        tracing::info!("success");
        Ok(())
    }

    /// Start all network devices
    pub fn run(&mut self) -> Result<()> {
        tracing::info!("startup...");

        for dev in self.iter_mut_devices() {
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

        for dev in self.iter_mut_devices() {
            let dev_name = dev.name_string();
            dev.close()
                .with_context(|| format!("Failed to close device: {}", dev_name))?;
        }

        tracing::info!("success");
        Ok(())
    }
}
