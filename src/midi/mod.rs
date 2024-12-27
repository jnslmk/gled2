pub mod apc40_mk2;
pub mod input;
pub mod output;
pub mod state;

use std::thread::spawn;

pub fn start_thread() {
    spawn(state::start);
    spawn(input::discover);
    spawn(output::discover);
}
