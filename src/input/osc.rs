use std::{io, thread};
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;
use kanal::{bounded, Receiver, Sender};
use log::{debug, trace};
use rosc::{OscMessage, OscPacket};

static OSC_PORT: u16 = 8000;

pub struct OSCHandler{
    socket: UdpSocket,
    tx: Sender<(OscMessage, std::net::SocketAddr)>,
    rx: Receiver<(OscMessage, std::net::SocketAddr)>,
}

impl OSCHandler{
    pub fn start() -> io::Result<Arc<Self>>{
        let socket = UdpSocket::bind(("0.0.0.0", OSC_PORT))?;
        let (tx, rx) = bounded(100);
        let ret = Arc::new(Self{
            socket,
            tx,
            rx,
        });
        start_network_loop(ret.clone());
        Ok(ret)
    }
}

fn start_network_loop(handler: Arc<OSCHandler>){
    thread::Builder::new()
        .name("gled:artnet:osc".to_string())
        .spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];
            let tx = handler.tx.clone();
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
                        // match tx.send((msg, src)) {
                        //     Ok(_) => {}
                        //     Err(_) => {
                        //         warn!("OSCHandler got dropped; stopping listening to osc network");
                        //         return;
                        //     }
                        // }
                    }
                    OscPacket::Bundle(bundle) => {
                        trace!("OSC Bundle: {:?}", bundle);
                    }
                }
            }
        }).expect("Could not spawn osc receiver thread");
}