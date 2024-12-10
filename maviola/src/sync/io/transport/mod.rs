//! # 🔒 Synchronous transport implementations

mod file;
mod serial;
#[cfg(unix)]
mod sock;
mod tcp;
mod udp;
