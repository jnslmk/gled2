use kanal::{Sender, bounded};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::io;

use crate::storage::asset::project::Project;
use crate::storage::asset::scene::grid::GridLocation;

mod parse;
mod feedback;

static OSC_PORT: u16 = 8000;

pub struct OSCHandler {
    pub(super) socket: UdpSocket,
    state_sender: Sender<OscStateSnapshot>,
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
        let (state_sender, state_receiver) = bounded(1);
        let socket = UdpSocket::bind(("0.0.0.0", OSC_PORT))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        let ret = Arc::new(Self {
            socket,
            state_sender,
            has_subscribers: AtomicBool::new(false),
        });
        feedback::start_network_loop(ret.clone(), state_receiver);
        Ok(ret)
    }

    pub fn has_subscribers(&self) -> bool {
        self.has_subscribers.load(Ordering::Relaxed)
    }

    pub fn enqueue_state_snapshot(&self, snapshot: OscStateSnapshot) {
        if !self.has_subscribers() {
            return;
        }
        let _ = self.state_sender.try_send(snapshot);
    }
}
