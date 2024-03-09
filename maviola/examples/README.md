Basic Examples
==============

Synchronous API
---------------

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

Network
-------

Synchronous example of a node with multiple connections: [`network.rs`](network.rs)

```shell
cargo run --package maviola --example network
```

Asynchronous example of a node with multiple connections: [`async_network.rs`](async_network.rs)

```shell
cargo run --package maviola --example async_network
```

Message Signing
---------------

Basic synchronous example: [`message_signing.rs`](message_signing.rs)

```shell
cargo run --package maviola --example message_signing
```

Custom Processing
-----------------

The following examples shows, how to use custom message processors to scramble and unscramble frame data. In real-world
applications you might want to use proper encryption algorithms.

Synchronous scrambler: [`scrambler.rs`](scrambler.rs)

```shell
cargo run --package maviola --example scrambler
```
