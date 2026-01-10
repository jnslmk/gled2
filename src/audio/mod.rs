pub mod state;

use std::thread::spawn;

pub fn start_thread() {
    spawn(state::start);
}

