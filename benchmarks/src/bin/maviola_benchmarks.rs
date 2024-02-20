use std::alloc::System;
use std::thread;
use std::time::Duration;

#[cfg(feature = "mpmc")]
use maviola_benchmarks::mpmc::{benchmark_mpmc_broadcast, benchmark_mpmc_collect};
#[cfg(feature = "sync")]
use maviola_benchmarks::sync::benchmark_unix_sockets;

#[global_allocator]
static GLOBAL: maviola_benchmarks::trallocator::Trallocator<System> =
    maviola_benchmarks::trallocator::Trallocator::new(System);

#[allow(dead_code)]
fn debug_memory(name: &str, before: u64) {
    let immediate = GLOBAL.get() - before;

    thread::sleep(Duration::from_millis(100));
    let soon = GLOBAL.get() - before;

    log::info!("[{name}] memory used: {immediate} bytes, after 100ms: {soon} bytes",);
}

fn main() {
    GLOBAL.reset();

    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Trace) // Allow everything from current package
        .init();

    #[cfg(feature = "mpmc")]
    {
        {
            log::info!("[benchmark_mpmc_broadcast]");
            let base_mem = GLOBAL.get();
            benchmark_mpmc_broadcast(1_000, 1_000);
            debug_memory("benchmark_mpmc_broadcast", base_mem);
        }

        {
            log::info!("[benchmark_mpmc_collect]");
            let base_mem = GLOBAL.get();
            benchmark_mpmc_collect(1_000, 1_000);
            debug_memory("benchmark_mpmc_collect", base_mem);
        }
    }

    #[cfg(feature = "sync")]
    {
        log::info!("[benchmark_unix_sockets]");
        let base_mem = GLOBAL.get();
        benchmark_unix_sockets(100, 1_000);
        debug_memory("benchmark_unix_sockets", base_mem);
    }
}

#[cfg(test)]
mod benchmark_tests {
    #[test]
    #[cfg(feature = "mpmc")]
    fn run_benchmark_mpmc_collect() {
        super::benchmark_mpmc_collect(100, 100);
    }

    #[test]
    #[cfg(feature = "mpmc")]
    fn run_benchmark_mpmc_broadcast() {
        super::benchmark_mpmc_broadcast(100, 100);
    }

    #[test]
    #[cfg(feature = "sync")]
    fn run_benchmark_unix_sockets() {
        super::benchmark_unix_sockets(10, 10);
    }
}
