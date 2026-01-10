use egui::{Color32, RichText};
use std::sync::atomic::AtomicU16;

pub static TEMPERATURE: AtomicU16 = AtomicU16::new(0);

pub fn start_thread() {
    std::thread::spawn(move || {
        loop {
            #[cfg(feature = "profiling")]
            profiling::register_thread!("temperature");

            let components = sysinfo::Components::new_with_refreshed_list();
            let mut max_temperature = 0f32;
            for component in &components {
                if let Some(temperature) = component.temperature() {
                    max_temperature = max_temperature.max(temperature);
                }
            }
            TEMPERATURE.store(
                max_temperature.round() as u16,
                std::sync::atomic::Ordering::Relaxed,
            );
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

pub fn temperature() -> RichText {
    let temperature = TEMPERATURE.load(std::sync::atomic::Ordering::Relaxed);
    let mut text = RichText::new(format!("{temperature}°C"));
    if temperature >= 90 {
        text = text.color(Color32::RED);
    } else if temperature >= 80 {
        text = text.color(Color32::ORANGE);
    }
    text
}
