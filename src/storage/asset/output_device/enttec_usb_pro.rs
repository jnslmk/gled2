use crossbeam_channel::Sender;
use egui::mutex::Mutex;
use log::{debug, trace, warn};
use once_cell::sync::{Lazy, OnceCell};
use serialport::SerialPort;
use std::{
    collections::{HashMap, hash_map::Entry},
    thread,
    time::{Duration, Instant},
};

static SERIAL_NUMBERS_PORT: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static SENDER: OnceCell<Sender<(String, [u8; 512])>> = OnceCell::new();

pub fn start() {
    find_devices();

    let (tx, rx) = crossbeam_channel::unbounded();
    SENDER.set(tx).expect("SENDER already set");

    debug!("Spawning enttec dmx usb pro thread");
    thread::spawn(move || {
        let mut devices = HashMap::<String, Box<dyn SerialPort>>::new();
        loop {
            let now = Instant::now();
            let mut data_per_serial_number = HashMap::new();
            while let Ok((serial_number, data)) = rx.try_recv() {
                data_per_serial_number.insert(serial_number, data);
            }

            for (serial_number, data) in data_per_serial_number {
                let mut enttec_data = vec![
                    0x7E, // Start of message
                    6,    // DMX output message
                    1,    // Data length LSB
                    2,    // Data length MSB
                    0,    // DMX start code
                ];
                enttec_data.extend_from_slice(&data);
                enttec_data.push(0xE7); // End of message

                log::trace!("Sending data to enttec dmx usb pro: {enttec_data:?}");

                match devices.entry(serial_number) {
                    Entry::Occupied(mut occupied_entry) => {
                        if let Err(err) = occupied_entry.get_mut().write_all(&enttec_data) {
                            warn!("Could not write to port: {err}");
                            occupied_entry.remove();
                        } else {
                            trace!("Send data to enttec dmx usb pro");
                        }
                    }
                    Entry::Vacant(vacant_entry) => {
                        let Some(port) =
                            SERIAL_NUMBERS_PORT.lock().get(vacant_entry.key()).cloned()
                        else {
                            warn!(
                                "Could not find port for serial number {}",
                                vacant_entry.key()
                            );
                            continue;
                        };

                        match serialport::new(port, 115_200).open() {
                            Ok(mut port) => {
                                if let Err(err) = port.set_timeout(Duration::from_millis(50)) {
                                    warn!("Could not set timeout on port: {err:?}");
                                    continue;
                                }
                                if let Err(err) = port.write_all(&enttec_data) {
                                    warn!("Could not write to port: {err}");
                                } else {
                                    trace!("Send data to enttec dmx usb pro");
                                    vacant_entry.insert(port);
                                }
                            }
                            Err(err) => {
                                warn!("Could not open port: {err}");
                            }
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(1000 / 40).saturating_sub(now.elapsed()));
        }
    });
}

fn find_devices() {
    debug!("Spawning enttec dmx usb pro discovery thread");
    thread::spawn(|| {
        loop {
            let mut serial_numbers_port = HashMap::new();
            let ports = match serialport::available_ports() {
                Ok(ports) => ports,
                Err(err) => {
                    warn!("Error listing serial ports: {err}");
                    std::thread::sleep(Duration::from_secs(60));
                    continue;
                }
            };
            for port in ports {
                if let serialport::SerialPortType::UsbPort(usb_port_info) = port.port_type
                    && usb_port_info.manufacturer == Some("ENTTEC".to_owned())
                    && usb_port_info.product == Some("DMX USB PRO".to_owned())
                    && let Some(serial_number) = usb_port_info.serial_number
                {
                    serial_numbers_port.insert(serial_number, port.port_name);
                }
            }
            *SERIAL_NUMBERS_PORT.lock() = serial_numbers_port;
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

pub fn serial_numbers() -> Vec<String> {
    SERIAL_NUMBERS_PORT.lock().keys().cloned().collect()
}

pub fn send(serial_number: String, data: [u8; 512]) {
    if let Some(sender) = SENDER.get() {
        if let Err(err) = sender.send((serial_number, data)) {
            warn!("Could not send data to enttec dmx usb pro thread: {err}");
        }
    } else {
        warn!("enttec dmx usb pro thread not started");
    }
}
