use anyhow::Result;

use crate::{
    net::{self, Device},
    util::debugdump,
};

fn ip_input(data: &[u8], dev: &Device) {
    // Skeleton implementation
    tracing::debug!("ip_input: dev={}, len={}", dev.name_string(), data.len());
    debugdump(data);
}

pub fn init(net_stack: &mut net::NetStack) -> Result<()> {
    tracing::info!("Initializing IP protocol");
    net_stack.register_protocol(net::NET_PROTOCOL_TYPE_IP, ip_input)?;
    Ok(())
}
