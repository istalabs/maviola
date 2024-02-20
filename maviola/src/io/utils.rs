use std::net::{SocketAddr, ToSocketAddrs};

use crate::prelude::*;

/// Resolves socket address.
///
/// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
/// available.
pub(crate) fn resolve_socket_addr(addr: impl ToSocketAddrs) -> Result<SocketAddr> {
    let mut resolved_addr = None;
    if let Some(addr) = addr.to_socket_addrs()?.next() {
        if resolved_addr.is_none() || addr.is_ipv4() {
            resolved_addr = Some(addr);
        }
    }

    if let Some(addr) = resolved_addr {
        return Ok(addr);
    }

    Err(Error::from(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "cant's resolve provided socket address",
    )))
}

pub(crate) fn pick_unused_port() -> Result<portpicker::Port> {
    portpicker::pick_unused_port().ok_or(
        std::io::Error::new(std::io::ErrorKind::NotFound, "can't find an unused port").into(),
    )
}
