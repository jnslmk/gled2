use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use kanal::{Receiver, bounded};

const UPDATE_SECONDS: u64 = 3;
const COUNTED_BYTES_TO_MBIT: f64 = (UPDATE_SECONDS * 1024 * 1024 / 8) as f64;

static INCOMING_BYTES: AtomicUsize = AtomicUsize::new(0);
static OUTGOING_BYTES: AtomicUsize = AtomicUsize::new(0);

pub fn start_thread() -> Receiver<(f64, f64)> {
    let (tx, rx) = bounded(0);

    std::thread::Builder::new()
        .name("gled:network:stats".to_string())
        .spawn(move || {
            tracing::info!("Starting network stats thread");
            #[cfg(feature = "profiling")]
            profiling::register_thread!("network:stats");

            loop {
                std::thread::sleep(std::time::Duration::from_secs(UPDATE_SECONDS));

                let incoming = INCOMING_BYTES.swap(0, Relaxed);
                let outgoing = OUTGOING_BYTES.swap(0, Relaxed);
                tx.send((
                    incoming as f64 / COUNTED_BYTES_TO_MBIT,
                    outgoing as f64 / COUNTED_BYTES_TO_MBIT,
                ))
                .expect("Could not send network stats");
            }
        })
        .expect("Could not spawn network stats thread");

    rx
}

pub fn add_incoming_bytes(bytes: usize) {
    INCOMING_BYTES.fetch_add(bytes, Relaxed);
}

pub fn add_outgoing_bytes(bytes: usize) {
    OUTGOING_BYTES.fetch_add(bytes, Relaxed);
}
