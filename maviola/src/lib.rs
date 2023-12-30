//! # Maviola
//!
//! A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust. Maviola
//! provides abstractions like communication nodes and namespaces and takes care of **stateful**
//! features of the MAVLink protocol, such as sequencing, message time-stamping, automatic
//! heartbeats, simplifies message signing, and so on.

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc(
    html_logo_url = "https://gitlab.com/mavka/libs/maviola/-/raw/main/avatar.png?ref_type=heads",
    html_favicon_url = "https://gitlab.com/mavka/libs/maviola/-/raw/main/avatar.png?ref_type=heads"
)]

#[doc(inline = true)]
pub extern crate mavio;

#[doc(inline = true)]
pub use mavio::dialects;

#[cfg(test)]
mod tests {
    #[test]
    fn tests_are_ok() {
        assert!(true);
    }
}
