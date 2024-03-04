//! # 🔒 Synchronous transport implementations

mod file;
#[cfg(unix)]
mod sock;
mod tcp;
mod udp;
