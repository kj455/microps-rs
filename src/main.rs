pub mod net;

use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let terminate = Arc::new(AtomicBool::new(false));

    setup(Arc::clone(&terminate)).context("setup() failure")?;

    let ret = app_main(terminate);

    cleanup().context("cleanup() failure")?;
    ret
}

fn setup(terminate: Arc<AtomicBool>) -> Result<()> {
    // Setup signal handler for SIGINT
    ctrlc::set_handler(move || {
        terminate.store(true, Ordering::SeqCst);
    })
    .context("sigaction() failed")?;

    net::init().context("net::init() failure")?;
    net::run().context("net::run() failure")?;

    Ok(())
}

fn app_main(terminate: Arc<AtomicBool>) -> Result<()> {
    tracing::info!("Application main loop started. Press Ctrl+C to exit.");

    // Main loop
    while !terminate.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    tracing::info!("Termination signal received. Shutting down...");
    Ok(())
}

fn cleanup() -> Result<()> {
    net::shutdown().context("net_shutdown() failure")?;
    Ok(())
}
