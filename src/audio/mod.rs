pub mod device_id_serde;
pub mod fft;
pub mod sound_data;
pub mod sound_trigger;
pub mod sound_trigger_editor;

use crate::audio::{
    fft::{FFTAudioSource, FREQ_BINS},
    sound_data::SoundData,
    sound_trigger::{SoundTrigger, SoundTriggerParams},
};
use cpal::{
    DeviceDescription, DeviceId, InterfaceType,
    traits::{DeviceTrait, HostTrait},
};
use kanal::Sender;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    thread::sleep,
    time::Duration,
};
use uuid::Uuid;

static SOUND_TRIGGER_SAMPLE_INTERVAL_MS: u64 = 10;
pub static AUDIO_DEVICES: Lazy<Mutex<Vec<(DeviceId, DeviceDescription)>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub fn audio_device_labels(devices: &[(DeviceId, DeviceDescription)]) -> Vec<(DeviceId, String)> {
    let mut duplicate_names = HashMap::<&str, usize>::new();
    for (_, description) in devices {
        *duplicate_names.entry(description.name()).or_default() += 1;
    }

    let preliminary_labels = devices
        .iter()
        .map(|(device_id, description)| {
            (
                device_id.clone(),
                format_audio_device_label(
                    description.name(),
                    description.manufacturer(),
                    description.address(),
                    description.interface_type(),
                    description.driver(),
                    &description.extended().collect::<Vec<_>>(),
                    duplicate_names[description.name()] > 1,
                ),
            )
        })
        .collect::<Vec<_>>();

    let mut duplicate_labels = HashMap::<String, usize>::new();
    for (_, label) in &preliminary_labels {
        *duplicate_labels.entry(label.clone()).or_default() += 1;
    }

    let mut duplicate_index = HashMap::<String, usize>::new();
    preliminary_labels
        .into_iter()
        .map(|(device_id, label)| {
            if duplicate_labels[&label] <= 1 {
                return (device_id, label);
            }

            let index = duplicate_index
                .entry(label.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            (device_id, format!("{} ({})", label, index))
        })
        .collect()
}

pub fn audio_device_label(_device_id: &DeviceId, description: &DeviceDescription) -> String {
    format_audio_device_label(
        description.name(),
        description.manufacturer(),
        description.address(),
        description.interface_type(),
        description.driver(),
        &description.extended().collect::<Vec<_>>(),
        false,
    )
}

fn format_audio_device_label(
    name: &str,
    manufacturer: Option<&str>,
    address: Option<&str>,
    interface_type: InterfaceType,
    driver: Option<&str>,
    extended: &[&str],
    duplicate_name: bool,
) -> String {
    let primary = preferred_primary_label(name, manufacturer, extended);

    let detail = label_detail(
        &primary,
        manufacturer,
        address,
        interface_type,
        driver,
        extended,
        duplicate_name,
    );

    if let Some(detail) = detail {
        format!("{} ({})", primary, detail)
    } else {
        primary
    }
}

fn preferred_primary_label(name: &str, manufacturer: Option<&str>, extended: &[&str]) -> String {
    let normalized_name = normalize_label(name);
    let best_extended = preferred_extended_line(extended, &normalized_name);

    if is_generic_device_name(&normalized_name) {
        if let Some(best_extended) = best_extended {
            return best_extended;
        }

        if let Some(manufacturer) = manufacturer.and_then(sanitize_metadata_line) {
            if manufacturer.to_lowercase().contains("audio") {
                return manufacturer;
            }
            return format!("{} Audio", manufacturer);
        }
    }

    normalized_name
}

fn label_detail(
    primary_label: &str,
    manufacturer: Option<&str>,
    address: Option<&str>,
    interface_type: InterfaceType,
    driver: Option<&str>,
    extended: &[&str],
    duplicate_name: bool,
) -> Option<String> {
    if duplicate_name {
        if let Some(manufacturer) =
            manufacturer
                .and_then(sanitize_metadata_line)
                .filter(|manufacturer| {
                    !primary_label
                        .to_lowercase()
                        .contains(&manufacturer.to_lowercase())
                })
        {
            return Some(manufacturer);
        }

        if let Some(detail) = secondary_extended_line(extended, primary_label) {
            return Some(detail);
        }

        if let Some(address) = address.and_then(pretty_address) {
            return Some(address);
        }

        if interface_type != InterfaceType::Unknown {
            return Some(interface_type.to_string());
        }

        if let Some(driver) = driver.and_then(sanitize_metadata_line) {
            return Some(driver);
        }
    }

    None
}

fn sanitize_metadata_line(text: &str) -> Option<String> {
    let normalized = normalize_label(text);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn preferred_extended_line(extended: &[&str], name: &str) -> Option<String> {
    extended
        .iter()
        .filter_map(|line| sanitize_metadata_line(line))
        .find(|line| is_useful_extended_line(line, name))
}

fn secondary_extended_line(extended: &[&str], primary_label: &str) -> Option<String> {
    extended
        .iter()
        .filter_map(|line| sanitize_metadata_line(line))
        .find(|line| {
            !line.eq_ignore_ascii_case(primary_label)
                && !line.eq_ignore_ascii_case("Direct hardware device without any conversions")
                && !line.eq_ignore_ascii_case("Default Audio Device")
        })
}

fn is_useful_extended_line(line: &str, name: &str) -> bool {
    !line.eq_ignore_ascii_case(name)
        && !line.eq_ignore_ascii_case("Direct hardware device without any conversions")
        && !line.eq_ignore_ascii_case("Default Audio Device")
}

fn is_generic_device_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "default"
        || lower == "default audio device"
        || lower == "audio"
        || lower == "usb audio"
        || lower.starts_with("monitor of")
}

fn normalize_label(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pretty_address(address: &str) -> Option<String> {
    let normalized = normalize_label(address);
    if normalized.is_empty() {
        return None;
    }

    if let Some(value) = normalized.strip_prefix("hw:") {
        let parts = value.split(',').collect::<Vec<_>>();
        if parts.len() == 2 {
            return Some(format!("Card {}, Device {}", parts[0], parts[1]));
        }
    }

    (normalized.len() <= 24).then_some(normalized)
}

#[derive(Debug, Default)]
pub struct AudioPool {
    pub selected_device: Option<DeviceId>,
    pub audio_source: Option<FFTAudioSource>,
}

impl AudioPool {
    pub fn restart_fft(&mut self) {
        self.audio_source = self.selected_device.clone().map(|device_id| {
            tracing::info!("Restarting FFT with device: {:?}", device_id);
            let fft_tx = start_process_fft_data_thread();
            FFTAudioSource::start(device_id, fft_tx)
        });
        let mut data = SoundData::default();
        data.reset();
        data.save();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoundTriggerHandle {
    uuid: Uuid,
}

impl Drop for SoundTriggerHandle {
    fn drop(&mut self) {
        let mut data = SoundData::default();
        data.remove(self.uuid);
        data.save();
    }
}

impl PartialEq for SoundTriggerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl SoundTriggerHandle {
    pub fn get_sound_trigger<'a>(&'a self, sound_data: &'a SoundData) -> Option<&'a SoundTrigger> {
        sound_data.triggers.get(&self.uuid)
    }

    pub fn get_sound_trigger_mut<'a>(
        &'a self,
        sound_data: &'a mut SoundData,
    ) -> Option<&'a mut SoundTrigger> {
        sound_data.triggers.get_mut(&self.uuid)
    }

    pub fn get_params<'a>(&'a self, sound_data: &'a SoundData) -> Option<&'a SoundTriggerParams> {
        Some(&self.get_sound_trigger(sound_data)?.params)
    }

    pub fn get_params_mut<'a>(
        &'a self,
        sound_data: &'a mut SoundData,
    ) -> Option<&'a mut SoundTriggerParams> {
        Some(&mut self.get_sound_trigger_mut(sound_data)?.params)
    }

    pub fn level(&self, sound_data: &SoundData) -> f32 {
        sound_data
            .triggers
            .get(&self.uuid)
            .map(|trigger| trigger.current_level)
            .unwrap_or(0.0)
    }
}

impl Hash for SoundTriggerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

pub fn start_process_fft_data_thread() -> Sender<[f32; FREQ_BINS]> {
    let (tx, rx) = kanal::bounded(0);

    std::thread::Builder::new()
        .name("gled:audio:trigger".to_string())
        .spawn(move || {
            #[cfg(feature = "profiling")]
            profiling::register_thread!("SoundTriggerThread");
            tracing::info!("Sound trigger thread started");
            let mut sound_data = SoundData::default();

            while let Ok(samples) = rx.recv() {
                sound_data.update();
                sound_data.tick(samples);
                sound_data.save();
            }
        })
        .expect("Could not spawn sound trigger thread");

    tx
}

// poll for audio device changes every second
pub fn audio_device_info_loop(continue_scan: &AtomicBool) {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio_device_info_loop");
    tracing::info!("Started scanning for audio devices...");
    while continue_scan.load(Relaxed) {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("audio_device_info_loop");

        let host = cpal::default_host();
        let devices = host.input_devices().expect("Failed to get audio devices");
        let device_map = Vec::from_iter(devices.filter_map(|device| {
            let id = device.id().ok()?;
            let description = device.description().ok()?;
            if !is_capture_input_device(&description) {
                return None;
            }

            Some((id, description))
        }));

        *AUDIO_DEVICES.lock().expect("audio devices lock poisoned") = device_map;
        sleep(Duration::from_secs(1));
    }
    tracing::info!("Audio devices scanning stopped.");
}

fn is_capture_input_device(description: &DeviceDescription) -> bool {
    if !description.supports_input() {
        return false;
    }

    let name = description.name().to_lowercase();
    if name.contains("monitor of") {
        return false;
    }

    if description
        .extended()
        .any(|line| line.to_lowercase().contains("monitor of"))
    {
        return false;
    }

    // On Linux the driver field can identify pseudo-devices.
    // Filter common ALSA virtual/plugin sources while keeping PipeWire,
    // which can represent real microphones on modern Linux desktops.
    #[cfg(target_os = "linux")]
    if let Some(driver) = description.driver() {
        let driver = driver.to_lowercase();
        let is_pipewire = driver == "pipewire" || driver.starts_with("pipewire:");
        let is_virtual_driver = driver == "default"
            || driver == "pulse"
            || driver.starts_with("sysdefault:")
            || driver.starts_with("dsnoop:")
            || driver.starts_with("iec958:")
            || driver.starts_with("dmix:")
            || driver.starts_with("usbstream:");

        if is_virtual_driver && !is_pipewire {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::DeviceDescriptionBuilder;
    use cpal::DeviceDirection;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::str::FromStr;
    use std::sync::atomic::Ordering::Relaxed;
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::{Duration, Instant};

    #[test]
    fn format_audio_device_label_prefers_manufacturer_and_address() {
        let label = format_audio_device_label(
            "USB Audio",
            Some("Focusrite"),
            Some("hw:2,0"),
            InterfaceType::Usb,
            Some("ALSA"),
            &[],
            true,
        );

        assert_eq!(label, "Focusrite Audio (Card 2, Device 0)");
    }

    #[test]
    fn format_audio_device_label_uses_pretty_address_for_duplicate_names() {
        let label = format_audio_device_label(
            "USB Audio",
            None,
            Some("hw:2,0"),
            InterfaceType::Unknown,
            None,
            &[],
            true,
        );

        assert_eq!(label, "USB Audio (Card 2, Device 0)");
    }

    #[test]
    fn format_audio_device_label_uses_driver_when_other_metadata_is_missing() {
        let label = format_audio_device_label(
            "USB Audio",
            None,
            None,
            InterfaceType::Unknown,
            Some("ALSA"),
            &[],
            true,
        );

        assert_eq!(label, "USB Audio (ALSA)");
    }

    #[test]
    fn format_audio_device_label_prefers_extended_human_description() {
        let description = DeviceDescriptionBuilder::new("USB Audio")
            .manufacturer("Focusrite Audio")
            .address("hw:2,0")
            .extended(vec![
                "USB Audio".to_string(),
                "Scarlett 2i2 USB Audio Interface".to_string(),
            ])
            .build();

        let label = format_audio_device_label(
            description.name(),
            description.manufacturer(),
            description.address(),
            description.interface_type(),
            description.driver(),
            &description.extended().collect::<Vec<_>>(),
            true,
        );

        assert_eq!(label, "Scarlett 2i2 USB Audio Interface (Focusrite Audio)");
    }

    #[test]
    fn audio_device_labels_add_number_suffix_for_identical_final_labels() {
        let devices = vec![
            (
                DeviceId::from_str("ALSA://device/1").expect("valid device id"),
                DeviceDescriptionBuilder::new("USB Audio").build(),
            ),
            (
                DeviceId::from_str("ALSA://device/2").expect("valid device id"),
                DeviceDescriptionBuilder::new("USB Audio").build(),
            ),
        ];

        let labels = audio_device_labels(&devices)
            .into_iter()
            .map(|(_, label)| label)
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["USB Audio (1)", "USB Audio (2)"]);
    }

    #[test]
    fn is_capture_input_device_accepts_real_input_device() {
        let description = DeviceDescriptionBuilder::new("Built-in Microphone")
            .direction(DeviceDirection::Input)
            .build();

        assert!(is_capture_input_device(&description));
    }

    #[test]
    fn is_capture_input_device_accepts_duplex_device() {
        let description = DeviceDescriptionBuilder::new("USB Audio Codec")
            .direction(DeviceDirection::Duplex)
            .build();

        assert!(is_capture_input_device(&description));
    }

    #[test]
    fn is_capture_input_device_rejects_monitor_sources() {
        let description = DeviceDescriptionBuilder::new("Monitor of Built-in Audio")
            .direction(DeviceDirection::Input)
            .extended(vec!["Monitor of sink output".to_string()])
            .build();

        assert!(!is_capture_input_device(&description));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn is_capture_input_device_rejects_alsa_virtual_pseudo_devices() {
        for pseudo_driver in &[
            "default",
            "pulse",
            "sysdefault:CARD=PCH",
            "dsnoop:CARD=PCH,DEV=0",
            "iec958:CARD=PCH",
        ] {
            let description = DeviceDescriptionBuilder::new("Some Device")
                .direction(DeviceDirection::Input)
                .driver(*pseudo_driver)
                .build();
            assert!(
                !is_capture_input_device(&description),
                "should reject ALSA pseudo-device with driver={pseudo_driver}"
            );
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn is_capture_input_device_accepts_pipewire_mic_device() {
        let description = DeviceDescriptionBuilder::new("Built-in Microphone")
            .direction(DeviceDirection::Input)
            .driver("pipewire")
            .build();

        assert!(
            is_capture_input_device(&description),
            "should accept real mic exposed via PipeWire"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn is_capture_input_device_accepts_hw_addressed_devices() {
        for driver in &["hw:0,0", "hw:1,0", "plughw:0,0"] {
            let description = DeviceDescriptionBuilder::new("Built-in Mic")
                .direction(DeviceDirection::Input)
                .driver(*driver)
                .build();
            assert!(
                is_capture_input_device(&description),
                "should accept real device with driver={driver}"
            );
        }
    }

    #[test]
    #[ignore = "Requires a live microphone/audio input device"]
    fn live_audio_input_stream_receives_samples() {
        let host = cpal::default_host();
        let default_input_id = host
            .default_input_device()
            .and_then(|device| device.id().ok());

        let mut candidates = host
            .input_devices()
            .expect("Failed to enumerate input devices")
            .filter_map(|device| {
                let description = device.description().ok()?;
                if !is_capture_input_device(&description) {
                    return None;
                }
                if device.default_input_config().is_err() {
                    return None;
                }
                Some(device)
            })
            .collect::<Vec<_>>();

        if let Some(default_id) = default_input_id {
            candidates.sort_by_key(|device| {
                if device.id().ok().as_ref() == Some(&default_id) {
                    0
                } else {
                    1
                }
            });
        }

        assert!(
            !candidates.is_empty(),
            "No usable input device found. Connect/select a microphone and retry."
        );

        let mut attempts = Vec::new();

        for device in candidates {
            let label = device
                .description()
                .map(|description| {
                    audio_device_label(&device.id().expect("Missing device id"), &description)
                })
                .unwrap_or_else(|_| "Unknown input device".to_string());

            let config = device
                .default_input_config()
                .expect("Input device has no default input config");
            let stream_config = cpal::StreamConfig::from(config);

            let got_samples = Arc::new(AtomicBool::new(false));
            let got_non_silent_signal = Arc::new(AtomicBool::new(false));
            let stream_error = Arc::new(AtomicBool::new(false));

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    let got_samples = got_samples.clone();
                    let got_non_silent_signal = got_non_silent_signal.clone();
                    let stream_error = stream_error.clone();
                    device.build_input_stream(
                        stream_config,
                        move |data: &[f32], _| {
                            if !data.is_empty() {
                                got_samples.store(true, Relaxed);
                            }
                            if data.iter().any(|sample| sample.abs() > 0.0) {
                                got_non_silent_signal.store(true, Relaxed);
                            }
                        },
                        move |_| {
                            stream_error.store(true, Relaxed);
                        },
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    let got_samples = got_samples.clone();
                    let got_non_silent_signal = got_non_silent_signal.clone();
                    let stream_error = stream_error.clone();
                    device.build_input_stream(
                        stream_config,
                        move |data: &[i16], _| {
                            if !data.is_empty() {
                                got_samples.store(true, Relaxed);
                            }
                            if data.iter().any(|sample| *sample != 0) {
                                got_non_silent_signal.store(true, Relaxed);
                            }
                        },
                        move |_| {
                            stream_error.store(true, Relaxed);
                        },
                        None,
                    )
                }
                cpal::SampleFormat::I32 => {
                    let got_samples = got_samples.clone();
                    let got_non_silent_signal = got_non_silent_signal.clone();
                    let stream_error = stream_error.clone();
                    device.build_input_stream(
                        stream_config,
                        move |data: &[i32], _| {
                            if !data.is_empty() {
                                got_samples.store(true, Relaxed);
                            }
                            if data.iter().any(|sample| *sample != 0) {
                                got_non_silent_signal.store(true, Relaxed);
                            }
                        },
                        move |_| {
                            stream_error.store(true, Relaxed);
                        },
                        None,
                    )
                }
                cpal::SampleFormat::U16 => {
                    let got_samples = got_samples.clone();
                    let got_non_silent_signal = got_non_silent_signal.clone();
                    let stream_error = stream_error.clone();
                    device.build_input_stream(
                        stream_config,
                        move |data: &[u16], _| {
                            if !data.is_empty() {
                                got_samples.store(true, Relaxed);
                            }
                            if data.iter().any(|sample| *sample != u16::MAX / 2) {
                                got_non_silent_signal.store(true, Relaxed);
                            }
                        },
                        move |_| {
                            stream_error.store(true, Relaxed);
                        },
                        None,
                    )
                }
                other => panic!("Unsupported sample format for test: {other:?}"),
            }
            .expect("Failed to build audio input stream");

            stream.play().expect("Failed to start audio input stream");

            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(2) {
                if stream_error.load(Relaxed) {
                    attempts.push(format!("{label}: stream error"));
                    break;
                }
                if got_samples.load(Relaxed) && got_non_silent_signal.load(Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }

            if !stream_error.load(Relaxed) {
                let state = if !got_samples.load(Relaxed) {
                    "no samples"
                } else {
                    "samples but silent"
                };
                attempts.push(format!("{label}: {state}"));
            }
        }

        panic!(
            "No input device produced non-silent signal. Device attempts: {}. Ensure microphone permissions are granted, the mic is not muted, and speak/tap near the mic during the test.",
            attempts.join(" | ")
        );
    }
}
