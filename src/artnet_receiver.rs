use artnet_protocol::{ArtCommand, PortAddress};
use log::{debug, info};
use std::{
    net::UdpSocket,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
    thread,
    time::Duration,
};

pub enum ArtnetEvent {
    On { channel: u8 },
    Off { channel: u8 },
}

pub static ARTNET_CONFIG: Mutex<ArtnetConfig> = Mutex::new(ArtnetConfig {
    universe: 0,
    start: 1,
    channels: 10,
});

pub struct ArtnetConfig {
    pub universe: u16,
    pub start: u16,
    pub channels: u16,
}

impl ArtnetConfig {
    fn port_address(&self) -> PortAddress {
        PortAddress::try_from(self.universe).unwrap_or_else(|_| PortAddress::from(0))
    }
}

pub fn start_thread() -> Receiver<ArtnetEvent> {
    let (sender, receiver) = std::sync::mpsc::channel();

    info!("Spawning worker thread");
    thread::Builder::new()
        .name("gled:artnet:rx".to_owned())
        .spawn(move || {
            thread(sender);
        })
        .expect("Could not spawn artnet receive thread");

    receiver
}

fn thread(sender: Sender<ArtnetEvent>) {
    let socket = UdpSocket::bind(("0.0.0.0", 6454)).unwrap();
    let mut buf = [0; 4096];
    let mut previous = vec![0; 512];
    loop {
        let Ok((size, _src)) = socket.recv_from(&mut buf) else {
            debug!("Could not receive on artnet");
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };

        let Ok(ArtCommand::Output(output)) = ArtCommand::from_buffer(&buf[..size]) else {
            debug!("Could not parse artnet");
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };

        let config = ARTNET_CONFIG.lock().expect("ARTNET_CONFIG is poisoned");
        if output.port_address != config.port_address() {
            debug!("Ignoring universe {:?}", output.port_address);
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        let data = output.data.as_ref();

        for i in 0..config.channels as usize {
            let channel = config.start as usize + i - 1;
            if previous[channel] != data[channel] {
                sender
                    .send(if data[channel] > 127 {
                        ArtnetEvent::On { channel: i as u8 }
                    } else {
                        ArtnetEvent::Off { channel: i as u8 }
                    })
                    .expect("Could not send event");
            }
        }

        previous = data.to_owned();
    }
}
