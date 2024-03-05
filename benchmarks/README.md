Maviola Benchmarks
==================

Run all benchmarks:

```shell
cargo run --package maviola_benchmarks --bin maviola_benchmarks --all-features
```

Synchronous API
---------------

```shell
cargo run --package maviola_benchmarks --bin maviola_benchmarks --features sync
```

Asynchronous API
---------------

```shell
cargo run --package maviola_benchmarks --bin maviola_benchmarks --features async
```

MPMC
----

Since Maviola uses a custom Multiple Producers / Multiple Consumers channel for sending data between connections, we've
decided to track basic benchmarks for this module. 

Run:

```shell
cargo run --package maviola_benchmarks --bin maviola_benchmarks --features mpmc
```
