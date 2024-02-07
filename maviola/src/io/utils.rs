use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs};

use crate::errors::{Error, Result};

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
        ErrorKind::InvalidInput,
        "cant's resolve provided socket address",
    )))
}
