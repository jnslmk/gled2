mod apc40_mk2;
mod input;
mod output;
mod state;

use std::thread::spawn;

pub use state::MidiState;

pub fn start_thread() {
    spawn(state::start);
    spawn(input::discover);
    spawn(output::discover);
}
