use crate::audio::{AUDIO_DEVICES, FFT_THREAD};
use egui::Ui;

#[derive(Default)]
pub struct SettingsEditor{}

impl SettingsEditor{
    pub fn show(ui: &mut Ui){
        #[cfg(feature = "profiling")]
        puffin::profile_function!("SettingsEditor::show");

        let mut selected = FFT_THREAD.lock().unwrap().selected_device.clone();
        let old_selected = selected.clone();
        egui::ComboBox::from_label("Audio input device")
            .selected_text(format!("{:?}", selected))
            .show_ui(ui, |ui| {
                let devices = AUDIO_DEVICES.lock().unwrap().clone();
                for (id, desc) in devices {
                    ui.selectable_value(&mut selected, Some(id), desc.name());
                }
            });
        if selected != old_selected {
            log::info!("Audio input device changed to {:?}", selected);
            {
                let mut fft_thread = FFT_THREAD.lock().unwrap();
                fft_thread.selected_device = selected.clone();
                fft_thread.restart_fft();
            }
        }

    }
}