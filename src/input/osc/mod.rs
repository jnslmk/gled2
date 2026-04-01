use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::storage::asset::project::Project;
use crate::storage::asset::scene::grid::GridLocation;

mod feedback;
mod parse;

static OSC_PORT: u16 = 8000;

pub struct OSCHandler {
    pub(super) socket: UdpSocket,
    pub(super) has_subscribers: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct OscStateSnapshot {
    pub project: Option<Project>,
    pub selected_scene_instance: GridLocation,
    pub blackout: bool,
    pub beats_per_minute: f32,
    pub beat_progression: f32,
}

impl OSCHandler {
    pub fn start() -> io::Result<Arc<Self>> {
        let socket = UdpSocket::bind(("0.0.0.0", OSC_PORT))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        let ret = Arc::new(Self {
            socket,
            has_subscribers: AtomicBool::new(false),
        });
        feedback::start_network_loop(ret.clone());
        Ok(ret)
    }

    pub fn has_subscribers(&self) -> bool {
        self.has_subscribers.load(Ordering::Relaxed)
    }
}
