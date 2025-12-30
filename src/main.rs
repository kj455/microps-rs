pub mod context;
pub mod device;
pub mod iface;
pub mod protocol;
pub mod util;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::context::ProtocolContexts;
use crate::device::loopback::OutputCallback;
use crate::device::{DeviceIndex, DeviceManager};
use crate::iface::{IpIface, NetIface};
use crate::protocol::{PROTOCOL_TYPE_IP, ProtocolManager};

const MAIN_LOOP_INTERVAL: Duration = Duration::from_secs(1);

const TEST_ICMP_PACKET: &[u8] = &[
    0x45, 0x00, 0x00, 0x30, 0x00, 0x80, 0x00, 0x00, 0xff, 0x01, 0xbd, 0x4a, 0x7f, 0x00, 0x00, 0x01,
    0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x35, 0x64, 0x00, 0x80, 0x00, 0x01, 0x31, 0x32, 0x33, 0x34,
    0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5e, 0x26, 0x2a, 0x28, 0x29,
];

type SharedDeviceManager = Rc<RefCell<DeviceManager>>;
type SharedProtocolManager = Rc<RefCell<ProtocolManager>>;
type SharedProtocolContexts = Rc<RefCell<ProtocolContexts>>;

struct App {
    devices: SharedDeviceManager,
    protocols: SharedProtocolManager,
    ctx: SharedProtocolContexts,
    terminate: Arc<AtomicBool>,
    loopback_index: DeviceIndex,
}

impl App {
    fn new() -> Result<Self> {
        let terminate = Arc::new(AtomicBool::new(false));
        let devices = Rc::new(RefCell::new(DeviceManager::new()));
        let protocols = Rc::new(RefCell::new(ProtocolManager::new()));
        let ctx = Rc::new(RefCell::new(ProtocolContexts::new()));

        Self::setup_signal_handler(Arc::clone(&terminate))?;

        protocols
            .borrow_mut()
            .init()
            .context("Failed to initialize protocols")?;

        let loopback_index = Self::setup_loopback(&devices, &protocols, &ctx)?;

        devices
            .borrow_mut()
            .run()
            .context("Failed to start devices")?;

        Ok(Self {
            devices,
            protocols,
            ctx,
            terminate,
            loopback_index,
        })
    }

    fn run(&self) -> Result<()> {
        tracing::info!("Application started. Press Ctrl+C to exit.");

        while !self.terminate.load(Ordering::SeqCst) {
            self.send_test_packet()?;
            std::thread::sleep(MAIN_LOOP_INTERVAL);
        }

        tracing::info!("Shutting down...");
        Ok(())
    }

    fn setup_signal_handler(terminate: Arc<AtomicBool>) -> Result<()> {
        ctrlc::set_handler(move || {
            terminate.store(true, Ordering::SeqCst);
        })
        .context("Failed to set signal handler")
    }

    fn setup_loopback(
        devices: &SharedDeviceManager,
        protocols: &SharedProtocolManager,
        ctx: &SharedProtocolContexts,
    ) -> Result<DeviceIndex> {
        let devices_for_cb = Rc::clone(devices);
        let protocols_for_cb = Rc::clone(protocols);
        let ctx_for_cb = Rc::clone(ctx);

        let callback: OutputCallback = Rc::new(move |type_, data, index| {
            let devices = devices_for_cb.borrow();
            let protocols = protocols_for_cb.borrow();
            let ctx = ctx_for_cb.borrow();

            if let Some(dev) = devices.get(index) {
                protocols.dispatch(type_, data, dev, &ctx);
            }
        });

        let index = device::loopback::init(&mut devices.borrow_mut(), callback)
            .context("Failed to initialize loopback device")?;

        let ip_iface =
            IpIface::new("127.0.0.1", "255.0.0.0").context("Failed to create IP interface")?;

        if let Some(dev) = devices.borrow_mut().get_mut(index) {
            dev.register_iface(NetIface::Ip(ip_iface))
                .context("Failed to register IP interface")?;
        }

        Ok(index)
    }

    fn send_test_packet(&self) -> Result<()> {
        let devices = self.devices.borrow();
        let dev = devices
            .get(self.loopback_index)
            .ok_or_else(|| anyhow::anyhow!("Invalid device index: {}", self.loopback_index))?;

        dev.output(PROTOCOL_TYPE_IP, TEST_ICMP_PACKET, None)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(e) = self.devices.borrow_mut().shutdown() {
            tracing::error!("Shutdown failed: {:?}", e);
        }
    }
}

fn main() -> Result<()> {
    init_logging();

    let app = App::new().context("Failed to initialize app")?;
    app.run()
}

fn init_logging() {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .init();
}
