use std::fmt::{Debug, Formatter};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::SystemTime;

use maviola::io::sync::mpmc;

const PAYLOAD_SIZE: usize = 255;

#[derive(Copy, Clone)]
struct Payload([u8; PAYLOAD_SIZE]);

impl Payload {
    fn new(discriminator: usize) -> Self {
        Self([(discriminator % 256) as u8; PAYLOAD_SIZE])
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self([0; PAYLOAD_SIZE])
    }
}

impl Debug for Payload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(format!("Payload([u8; {PAYLOAD_SIZE}])").as_str())
            .finish_non_exhaustive()
    }
}

pub fn benchmark_mpmc_broadcast(n_receivers: usize, n_iter: usize) {
    let (tx, rx) = mpmc::channel();
    let (collect_tx, collect_rx) = mpsc::channel();

    let start_pair = Arc::new((Mutex::new(false), Condvar::new()));

    for _ in 0..n_receivers {
        let rx = rx.clone();
        let start_pair = start_pair.clone();
        let n_iter = n_iter.clone();
        let collect_tx = collect_tx.clone();

        thread::spawn(move || {
            let (lock, cvar) = &*start_pair;
            let mut started = lock.lock().unwrap();
            while !*started {
                started = cvar.wait(started).unwrap();
            }

            for _ in 0..n_iter {
                rx.recv().unwrap();
                collect_tx.send(()).unwrap();
            }
        });
    }

    let start = SystemTime::now();
    {
        let (lock, cvar) = &*start_pair;
        let mut started = lock.lock().unwrap();
        *started = true;
        cvar.notify_all();
    }

    for i in 0..n_iter {
        tx.send(Payload::new(i)).unwrap();
    }

    for _ in 0..n_iter * n_receivers {
        collect_rx.recv().unwrap();
    }

    let end = SystemTime::now();

    let duration = end.duration_since(start).unwrap();
    log::info!(
        "[benchmark_mpmc_broadcast] send {n_iter} of {:?} to {n_receivers} receivers: {}",
        Payload::default(),
        duration.as_secs_f32()
    )
}

pub fn benchmark_mpmc_collect(n_threads: usize, n_senders: usize) {
    let (tx, rx) = mpmc::channel();

    let start_pair = Arc::new((Mutex::new(false), Condvar::new()));

    for _ in 0..n_threads {
        let tx = tx.clone();
        let start_pair = start_pair.clone();

        thread::spawn(move || {
            let (lock, cvar) = &*start_pair;
            let mut started = lock.lock().unwrap();
            while !*started {
                started = cvar.wait(started).unwrap();
            }

            for i in 0..n_senders {
                let tx = tx.clone();
                tx.send(Payload::new(i)).unwrap();
            }
        });
    }

    let start = SystemTime::now();
    {
        let (lock, cvar) = &*start_pair;
        let mut started = lock.lock().unwrap();
        *started = true;
        cvar.notify_all();
    }

    for _ in 0..n_threads * n_senders {
        rx.recv().unwrap();
    }

    let end = SystemTime::now();

    let duration = end.duration_since(start).unwrap();
    log::info!(
        "[benchmark_mpmc_collect] {n_senders} per {n_threads} sending {:?} to a single MPMC receiver: {}s",
        Payload::default(),
        duration.as_secs_f32()
    )
}
