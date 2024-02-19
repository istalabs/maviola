Maviola
=======

A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust. Maviola provides abstractions like
communication nodes and namespaces and takes care of **stateful** features of the MAVLink protocol, such as sequencing,
message time-stamping, automatic heartbeats, simplifies message signing, and so on.

[🇺🇦](https://mavka.gitlab.io/home/a_note_on_the_war_in_ukraine/)
[`repository`](https://gitlab.com/mavka/libs/maviola)
[`crates.io`](https://crates.io/crates/maviola)
[`API docs`](https://docs.rs/maviola/latest/maviola/)
[`issues`](https://gitlab.com/mavka/libs/maviola/-/issues)

Maviola is based on [Mavio](https://gitlab.com/mavka/libs/mavio), a low-level library with `no-std` support. If you are
looking for a solution for embedded devices, then Mavio would be a better option.

> **⚠ WIP**
> 
> Maviola is still under heavy development. The aim is to provide API similar to
> [`gomavlib`](https://github.com/bluenviron/gomavlib) with additional support for essential MAVLink
> ["microservices"](https://mavlink.io/en/services/) such as [heartbeat](https://mavlink.io/en/services/heartbeat.html),
> [parameter protocol](https://mavlink.io/en/services/parameter.html) and
> [commands](https://mavlink.io/en/services/command.html).
> 
> We intentionally do not publish early versions of API to avoid confusion and massive breaking changes.

Examples
--------

Basic examples of client-server communication for different transports:

* TCP
  ```shell
  cargo run --package maviola --example tcp_ping_pong
  ```
* UDP
  ```shell
  cargo run --package maviola --example udp_ping_pong
  ```
* Unix sockets
  ```shell
  cargo run --package maviola --example sock_ping_pong
  ```

License
-------

> Here we simply comply with the suggested dual licensing according to
> [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/about.html) (C-PERMISSIVE).

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
