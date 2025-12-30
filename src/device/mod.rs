pub mod loopback;

use anyhow::{Context, Result};

use crate::iface::NetIface;
use crate::util::debugdump;

pub const IFNAMSIZ: usize = 16;
pub const NET_DEVICE_ADDR_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum DeviceType {
    #[default]
    Dummy = 0x0000,
    Loopback = 0x0001,
    Ethernet = 0x0002,
}

pub const NET_DEVICE_FLAG_UP: u16 = 0x0001;
pub const NET_DEVICE_FLAG_LOOPBACK: u16 = 0x0010;
pub const NET_DEVICE_FLAG_BROADCAST: u16 = 0x0020;
pub const NET_DEVICE_FLAG_P2P: u16 = 0x0040;
pub const NET_DEVICE_FLAG_NEED_ARP: u16 = 0x0100;

// Newtype pattern for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceIndex(pub usize);

impl std::fmt::Display for DeviceIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait DeviceOps {
    fn open(&self, dev: &Device) -> Result<()>;
    fn close(&self, dev: &Device) -> Result<()>;
    fn transmit(&self, dev: &Device, type_: u16, data: &[u8], dst: Option<&[u8]>) -> Result<()>;
}

pub struct Device {
    pub index: DeviceIndex,
    pub name: [u8; IFNAMSIZ],
    pub device_type: DeviceType,
    pub mtu: u16,
    pub flags: u16,
    pub hlen: u16,
    pub alen: u16,
    pub addr: [u8; NET_DEVICE_ADDR_LEN],
    pub broadcast: [u8; NET_DEVICE_ADDR_LEN],
    pub ops: Option<Box<dyn DeviceOps>>,
    pub ifaces: Vec<NetIface>,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            index: DeviceIndex::default(),
            name: [0; IFNAMSIZ],
            device_type: DeviceType::default(),
            mtu: 0,
            flags: 0,
            hlen: 0,
            alen: 0,
            addr: [0; NET_DEVICE_ADDR_LEN],
            broadcast: [0; NET_DEVICE_ADDR_LEN],
            ops: None,
            ifaces: Vec::new(),
        }
    }
}

impl Device {
    pub fn is_up(&self) -> bool {
        (self.flags & NET_DEVICE_FLAG_UP) != 0
    }

    pub fn state(&self) -> &str {
        if self.is_up() { "UP" } else { "DOWN" }
    }

    pub fn name_string(&self) -> String {
        String::from_utf8_lossy(&self.name)
            .trim_end_matches('\0')
            .to_string()
    }

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

        if let Some(ops) = &self.ops {
            ops.transmit(self, device_type, data, dst)?;
        }

        Ok(())
    }

    pub fn input(&self, type_: u16, data: &[u8]) -> Result<()> {
        tracing::debug!(
            "device_input: dev={}, type=0x{:04x}, len={}",
            self.name_string(),
            type_,
            data.len()
        );
        debugdump(data);
        Ok(())
    }

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

    pub fn register_iface(&mut self, iface: NetIface) -> Result<()> {
        let has_same_family = self
            .ifaces
            .iter()
            .any(|cur_iface| iface.family() == cur_iface.family());
        if has_same_family {
            anyhow::bail!("Interface family already registered: {:?}", iface.family());
        }

        match &iface {
            NetIface::Ip(ip_iface) => {
                tracing::info!("Registering IP interface: {}", ip_iface.info());
            }
        }

        self.ifaces.push(iface);
        Ok(())
    }

    pub fn get_ip_iface(&self) -> Option<&crate::iface::IpIface> {
        self.ifaces.iter().find_map(|iface| iface.as_ip())
    }
}

pub struct DeviceManager {
    devices: Vec<Device>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn register(&mut self, mut dev: Device) -> Result<DeviceIndex> {
        let index = DeviceIndex(self.devices.len());
        dev.index = index;

        let name_str = format!("net{}", index.0);
        let name_bytes = name_str.as_bytes();
        dev.name[..name_bytes.len()].copy_from_slice(name_bytes);

        tracing::info!(
            "Device registered: {}, type={:?}",
            name_str,
            dev.device_type
        );

        self.devices.push(dev);
        Ok(index)
    }

    pub fn get(&self, index: DeviceIndex) -> Option<&Device> {
        self.devices.get(index.0)
    }

    pub fn get_mut(&mut self, index: DeviceIndex) -> Option<&mut Device> {
        self.devices.get_mut(index.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Device> {
        self.devices.iter_mut()
    }

    pub fn run(&mut self) -> Result<()> {
        tracing::info!("Starting devices...");

        for dev in self.iter_mut() {
            let dev_name = dev.name_string();
            dev.open()
                .with_context(|| format!("Failed to open device: {}", dev_name))?;
        }

        tracing::info!("All devices started");
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down devices...");

        for dev in self.iter_mut() {
            let dev_name = dev.name_string();
            dev.close()
                .with_context(|| format!("Failed to close device: {}", dev_name))?;
        }

        tracing::info!("All devices stopped");
        Ok(())
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
