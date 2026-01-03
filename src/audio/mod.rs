pub mod state;
pub mod adsr_editor;
pub mod adsr;

use std::thread::spawn;

pub fn start_thread() {
    spawn(state::start);
}

