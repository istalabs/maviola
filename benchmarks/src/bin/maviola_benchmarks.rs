use std::alloc::System;
use std::thread;
use std::time::Duration;

#[cfg(feature = "mpmc")]
use maviola_benchmarks::mpmc::{benchmark_mpmc_broadcast, benchmark_mpmc_collect};

#[global_allocator]
static GLOBAL: maviola_benchmarks::trallocator::Trallocator<System> =
    maviola_benchmarks::trallocator::Trallocator::new(System);

#[allow(dead_code)]
fn debug_memory(name: &str, before: u64) {
    let immediate = GLOBAL.get() - before;

    thread::sleep(Duration::from_millis(1));
    let soon = GLOBAL.get() - before;

    log::info!("[{name}] memory used: {immediate} bytes, after 1ms: {soon} bytes",);
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
            let base_mem = GLOBAL.get();
            benchmark_mpmc_broadcast(1_000, 1_000);
            debug_memory("benchmark_mpmc_broadcast", base_mem);
        }

        {
            let base_mem = GLOBAL.get();
            benchmark_mpmc_collect(1_000, 1_000);
            debug_memory("benchmark_mpmc_collect", base_mem);
        }
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
}
