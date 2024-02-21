Basic Examples
==============

Basic examples of client-server communication for different transports:

* TCP: [`tcp_ping_pong.rs`](tcp_ping_pong.rs)
  ```shell
  cargo run --package maviola --example tcp_ping_pong
  ```
* UDP: [`udp_ping_pong.rs`](udp_ping_pong.rs)
  ```shell
  cargo run --package maviola --example udp_ping_pong
  ```
* Unix sockets: [`sock_ping_pong.rs`](sock_ping_pong.rs)
  ```shell
  cargo run --package maviola --example sock_ping_pong
  ```
* Read/write binary stream to a file: [`file_rw.rs`](file_rw.rs)
  ```shell
  cargo run --package maviola --example file_rw
  ```