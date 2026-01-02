use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

static STATS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
static INCOMING_BYTES: AtomicUsize = AtomicUsize::new(0);
static OUTGOING_BYTES: AtomicUsize = AtomicUsize::new(0);

pub fn start_thread() {
    std::thread::spawn(|| {
        log::info!("Starting network stats thread");
        #[cfg(feature = "profiling")]
        profiling::register_thread!("network:stats");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));

            let incoming = INCOMING_BYTES.swap(0, Relaxed);
            let outgoing = OUTGOING_BYTES.swap(0, Relaxed);
            let stats = format!(
                "In: {:.2} MBit/s, Out: {:.2} MBit/s",
                incoming as f64 / 393216.0,
                outgoing as f64 / 393216.0
            );
            *STATS.lock() = stats;
        }
    });
}

pub fn add_incoming_bytes(bytes: usize) {
    INCOMING_BYTES.fetch_add(bytes, Relaxed);
}

pub fn add_outgoing_bytes(bytes: usize) {
    OUTGOING_BYTES.fetch_add(bytes, Relaxed);
}

pub fn stats() -> String {
    STATS.lock().clone()
}
