use std::{io, thread};
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;
use log::{debug, trace, warn};
use rosc::{OscMessage, OscPacket};
use crate::ui::action::UiAction;

static OSC_PORT: u16 = 8000;

#[derive(Debug, Clone)]
pub enum ControlEvent {
    MainDimmer(f32),
}

pub struct OSCHandler{
    socket: UdpSocket,
}

impl OSCHandler{
    pub fn start() -> io::Result<Arc<Self>>{
        let socket = UdpSocket::bind(("0.0.0.0", OSC_PORT))?;
        let ret = Arc::new(Self{
            socket,
        });
        start_network_loop(ret.clone());
        Ok(ret)
    }
}

fn parse_message(msg: &OscMessage) -> Result<ControlEvent, ()>{
    match msg.addr.as_str() {
        "/main_dimmer" => {
            if let Some(v) = msg.args.first()
                .and_then(|v| v.clone().float()) {
                    Ok(ControlEvent::MainDimmer(v))
            } else {
                Err(())
            }
        },
        _ => {
            warn!("Received unknown osc message: {}", msg.addr);
            Err(())
        },
    }
}

fn start_network_loop(handler: Arc<OSCHandler>){
    thread::Builder::new()
        .name("gled:artnet:osc".to_string())
        .spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];
            trace!("Starting osc network receiving loop");
            loop {
                trace!("Waiting for osc package...");
                let Ok((size, src)) = handler.socket.recv_from(&mut buf) else {
                    debug!("Could not receive osc package. Retrying in 10ms...");
                    thread::sleep(Duration::from_millis(10));
                    continue;
                };

                trace!("Received packet with size {} from: {}", size, src);
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();

                match packet {
                    OscPacket::Message(msg) => {
                        trace!("OSC address: {}", msg.addr);
                        trace!("OSC arguments: {:?}", msg.args);
                        if let Ok(event) = parse_message(&msg) {
                            UiAction::enqueue(
                                UiAction::ProjectUpdate(event)
                            );
                        }
                    }
                    OscPacket::Bundle(bundle) => {
                        warn!("OSC Bundle are not supported currently: {:?}", bundle);
                    }
                }
            }
        }).expect("Could not spawn osc receiver thread");
}