use kanal::{Receiver, Sender, unbounded};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::{SystemTime, UNIX_EPOCH};

static STREAM_ENABLED: AtomicBool = AtomicBool::new(false);
static SENDER: OnceCell<Sender<MidiMonitorEvent>> = OnceCell::new();

#[derive(Clone, Debug, Default)]
pub struct MidiPortDiagnostics {
    pub port_name: String,
    pub input_connected: bool,
    pub output_connected: bool,
    pub input_connect_failed: u32,
    pub output_connect_failed: u32,
    pub message_count: u64,
}

#[derive(Clone, Debug)]
pub struct MidiMonitorEvent {
    pub timestamp_ms: u128,
    pub port_name: String,
    pub label: String,
    pub bytes: Vec<u8>,
}

pub fn init() -> Receiver<MidiMonitorEvent> {
    let (sender, receiver) = unbounded();
    let _ = SENDER.set(sender);
    receiver
}

pub fn set_streaming_enabled(enabled: bool) {
    STREAM_ENABLED.store(enabled, Relaxed);
}

pub fn push_event(port_name: &str, label: &str, bytes: &[u8]) {
    // Keep lifecycle diagnostics (connect/fail) even when the live stream is off.
    // Only raw midi traffic is gated to avoid filling the queue in the background.
    if label == "midi" && !STREAM_ENABLED.load(Relaxed) {
        return;
    }

    let event = MidiMonitorEvent {
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
        port_name: port_name.to_owned(),
        label: label.to_owned(),
        bytes: bytes.to_vec(),
    };

    if let Some(sender) = SENDER.get() {
        let _ = sender.send(event);
    }
}
