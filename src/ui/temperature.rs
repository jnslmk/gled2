use egui::{Color32, FontSelection, RichText, Ui};
use std::sync::atomic::AtomicU16;
use egui_phosphor_icons::icons;
use emath::Align;
use epaint::text::LayoutJob;

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

pub fn temperature(ui: &Ui) -> LayoutJob {
    let temperature = TEMPERATURE.load(std::sync::atomic::Ordering::Relaxed);
    let mut job = LayoutJob::default();
    icons::THERMOMETER.regular().append_to(&mut job, ui.style(), FontSelection::Default, Align::Center);
    RichText::new(format!(" {temperature}°C")).size(14.0)
        .append_to(&mut job, ui.style(), FontSelection::Default, Align::Center);

    if temperature >= 90 {
        for section in &mut job.sections {
            section.format.color = Color32::RED;
        }
    } else if temperature >= 80 {
        for section in &mut job.sections {
            section.format.color = Color32::ORANGE;
        }
    }
    job
}
