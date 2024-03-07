Basic Examples
==============

Synchronous API
---------------

### Basic examples

These examples show basic usage for different transports:

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

Asynchronous API
----------------

### Basic examples

These examples show basic usage for different transports:

* TCP: [`async_tcp_ping_pong.rs`](async_tcp_ping_pong.rs)
  ```shell
  cargo run --package maviola --example async_tcp_ping_pong
  ```
* Unix sockets: [`async_sock_ping_pong.rs`](async_sock_ping_pong.rs)
  ```shell
  cargo run --package maviola --example async_sock_ping_pong
  ```
* Read/write binary stream to a file: [`async_file_rw.rs`](async_file_rw.rs)
  ```shell
  cargo run --package maviola --example async_file_rw
  ```

Message Signing
---------------

Basic example:

```shell
cargo run --package maviola --example message_signing
```
