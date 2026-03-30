pub mod input;
pub mod learn;
pub mod monitor;
pub mod output;
pub mod runtime;
pub mod state;

use std::thread::spawn;

pub fn normalize_controller_key(port_name: &str) -> String {
    let trimmed = port_name.trim();
    if let Some(prefix) = trimmed.strip_suffix(']')
        && let Some((base, suffix)) = prefix.rsplit_once(" [")
        && !base.is_empty()
        && !suffix.is_empty()
        && suffix.chars().all(|ch| ch.is_ascii_digit())
    {
        return base.to_owned();
    }

    trimmed.to_owned()
}

pub fn start_thread() -> kanal::Receiver<monitor::MidiMonitorEvent> {
    let receiver = monitor::init();
    let runtime_bus = runtime::RuntimeBus::default();
    let input_runtime_bus = runtime_bus.clone();
    let output_runtime_bus = runtime_bus.clone();
    spawn(state::start);
    spawn(move || input::discover(input_runtime_bus));
    spawn(move || output::discover(output_runtime_bus));
    receiver
}
