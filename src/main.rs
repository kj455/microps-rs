pub mod net;

fn main() {
    tracing_subscriber::fmt().init();

    tracing::info!("Application started");

    net::init();
    net::run();
    net::shutdown();
}
