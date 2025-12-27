pub mod loopback;
pub mod net;
pub mod util;

use anyhow::{Context, Result};
use net::DeviceDescriptor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::EnvFilter;

// ICMP Echo Request packet (localhost to localhost)
// IPv4 header + ICMP header + payload data
const TEST_DATA: &[u8] = &[
    0x45, 0x00, 0x00, 0x30, 0x00, 0x80, 0x00, 0x00, 0xff, 0x01, 0xbd, 0x4a, 0x7f, 0x00, 0x00, 0x01,
    0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x35, 0x64, 0x00, 0x80, 0x00, 0x01, 0x31, 0x32, 0x33, 0x34,
    0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5e, 0x26, 0x2a, 0x28, 0x29,
];

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .init();

    let terminate = Arc::new(AtomicBool::new(false));

    let mut devices = net::NetDevices::new();

    let descriptor = setup(&mut devices, Arc::clone(&terminate)).context("setup() failure")?;

    let ret = app_main(terminate, &devices, descriptor);

    cleanup(&mut devices).context("cleanup() failure")?;
    ret
}

fn setup(devices: &mut net::NetDevices, terminate: Arc<AtomicBool>) -> Result<DeviceDescriptor> {
    // Setup signal handler for SIGINT
    ctrlc::set_handler(move || {
        terminate.store(true, Ordering::SeqCst);
    })
    .context("sigaction() failed")?;

    devices.init().context("net::init() failure")?;

    let descriptor = loopback::init(devices).context("loopback_init() failure")?;

    devices.run().context("net::run() failure")?;

    Ok(descriptor)
}

fn app_main(
    terminate: Arc<AtomicBool>,
    devices: &net::NetDevices,
    descriptor: DeviceDescriptor,
) -> Result<()> {
    tracing::info!("Application main loop started. Press Ctrl+C to exit.");

    // Main loop
    while !terminate.load(Ordering::SeqCst) {
        let dev = devices
            .get(descriptor)
            .ok_or_else(|| anyhow::anyhow!("Invalid device descriptor: {}", descriptor))?;

        dev.output(0x0800, TEST_DATA, None)?;

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    tracing::info!("Termination signal received. Shutting down...");
    Ok(())
}

fn cleanup(devices: &mut net::NetDevices) -> Result<()> {
    devices.shutdown().context("net_shutdown() failure")?;
    Ok(())
}
