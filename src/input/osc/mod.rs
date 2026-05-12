use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::storage::asset::project::Project;
use crate::storage::asset::scene::grid::GridLocation;

mod feedback;
mod parse;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct OscConfig {
    pub active: bool,
    pub port: u16,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            active: false,
            port: 8000,
        }
    }
}

pub struct OSCHandler {
    pub(super) socket: UdpSocket,
    pub(super) has_subscribers: AtomicBool,
    pub(super) running: AtomicBool,
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
    pub fn start(port: u16) -> io::Result<Arc<Self>> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        let ret = Arc::new(Self {
            socket,
            has_subscribers: AtomicBool::new(false),
            running: AtomicBool::new(true),
        });
        feedback::start_network_loop(ret.clone());
        Ok(ret)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn has_subscribers(&self) -> bool {
        self.has_subscribers.load(Ordering::Relaxed)
    }
}
