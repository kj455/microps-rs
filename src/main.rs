pub mod ip;
pub mod loopback;
pub mod net;
pub mod util;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::loopback::OutputCallback;
use crate::net::{DeviceDescriptor, NET_PROTOCOL_TYPE_IP, NetStack};

const MAIN_LOOP_INTERVAL: Duration = Duration::from_secs(1);

/// ICMP Echo Request packet (localhost to localhost)
/// IPv4 header + ICMP header + payload data
const TEST_ICMP_PACKET: &[u8] = &[
    0x45, 0x00, 0x00, 0x30, 0x00, 0x80, 0x00, 0x00, 0xff, 0x01, 0xbd, 0x4a, 0x7f, 0x00, 0x00, 0x01,
    0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x35, 0x64, 0x00, 0x80, 0x00, 0x01, 0x31, 0x32, 0x33, 0x34,
    0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5e, 0x26, 0x2a, 0x28, 0x29,
];

type SharedNetStack = Rc<RefCell<NetStack>>;

struct App {
    net_stack: SharedNetStack,
    terminate: Arc<AtomicBool>,
    loopback_descriptor: DeviceDescriptor,
}

impl App {
    fn new() -> Result<Self> {
        let terminate = Arc::new(AtomicBool::new(false));
        let net_stack = Rc::new(RefCell::new(NetStack::new()));

        Self::setup_signal_handler(Arc::clone(&terminate))?;
        let loopback_descriptor = Self::setup_loopback(&net_stack)?;
        net_stack.borrow_mut().run().context("net::run() failure")?;

        Ok(Self {
            net_stack,
            terminate,
            loopback_descriptor,
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

    fn setup_loopback(net_stack: &SharedNetStack) -> Result<DeviceDescriptor> {
        let net_stack_for_callback = Rc::clone(net_stack);
        let callback: OutputCallback = Rc::new(move |type_, data, descriptor| {
            let stack = net_stack_for_callback.borrow();
            if let Err(e) = stack.input(type_, data, descriptor) {
                tracing::error!("Failed to process input: {:?}", e);
            }
        });

        net_stack
            .borrow_mut()
            .init()
            .context("net::init() failure")?;

        loopback::init(&mut net_stack.borrow_mut(), callback).context("loopback::init() failure")
    }

    fn send_test_packet(&self) -> Result<()> {
        let stack = self.net_stack.borrow();
        let dev = stack.get_device(self.loopback_descriptor).ok_or_else(|| {
            anyhow::anyhow!("Invalid device descriptor: {}", self.loopback_descriptor)
        })?;

        dev.output(NET_PROTOCOL_TYPE_IP, TEST_ICMP_PACKET, None)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(e) = self.net_stack.borrow_mut().shutdown() {
            tracing::error!("Shutdown failed: {:?}", e);
        }
    }
}

// --- エントリポイント ---

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
