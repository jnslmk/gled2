use std::collections::{BTreeMap};
use crate::{
    storage::{
        asset::{Asset, output_device::OutputDevice},
        asset_id::AssetId,
    },
    ui::{ChangeButton, viewport_builder::default_viewport_builder, windows::channel_overwrites},
};
use egui::{
    Color32, ComboBox, Context, Id, Layout, RichText, Vec2, ViewportId, mutex::Mutex,
};
use once_cell::sync::Lazy;

pub struct ChannelOverwritesWindow {
    open: bool,
    device: Option<AssetId<OutputDevice>>,
    universe: Option<u16>,
    channel: u16,
    value: u8,
}

impl Default for ChannelOverwritesWindow {
    fn default() -> Self {
        Self {
            open: false,
            device: Default::default(),
            universe: Default::default(),
            channel: 1,
            value: Default::default(),
        }
    }
}

impl ChannelOverwritesWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("channel overwrites window")),
            default_viewport_builder()
                .with_title("Gled: Channel Overwrites")
                .with_inner_size(Vec2::new(600.0, 500.0))
                .with_min_inner_size(Vec2::new(600.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.close();
                    }
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut channel_overwrites = channel_overwrites::ChannelOverwrites::get();

                    ui.horizontal(|ui| {
                        if self.device.change_button(ui) {
                            self.universe.take();
                        }

                        if let Some(device) = self.device.and_then(Asset::get) {
                            let device_universes = device.data.universes();
                            if !device_universes.is_empty() {
                                ComboBox::new("channel_overwrites_universe", "")
                                    .selected_text(match self.universe {
                                        None => "No universe selected".to_string(),
                                        Some(universe) => format!("Universe {universe}"),
                                    })
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        for device_universe in device_universes {
                                            let res = ui.selectable_value(
                                                &mut self.universe,
                                                Some(*device_universe),
                                                format!("Universe {device_universe}"),
                                            );

                                            if res.changed() {
                                                self.universe = Some(*device_universe);
                                            }
                                        }
                                    });
                                }

                                ui.add(egui::DragValue::new(&mut self.channel).range(1..=512).prefix("Channel: "));
                                ui.add_enabled(device_universes.is_empty() || self.universe.is_some(), egui::DragValue::new(&mut self.value).range(0..=256).prefix("Value: "));
                                    if ui.button("Add/Update").on_hover_text(
                                        "Add or update the channel overwrite for the selected device, universe and channel",
                                    ).clicked() {
                                        channel_overwrites.0.insert(
                                            channel_overwrites::ChannelIdentifier {
                                                device: device.id,
                                                universe: self.universe,
                                                channel: self.channel,
                                            },
                                            self.value,
                                        );
                                        channel_overwrites.clone().set();
                                    }
                                
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("channel_overwrites_scroll")
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                            
                                for (channel_identifier, value) in channel_overwrites.0.iter() {
                                    ui.horizontal(|ui| {
                                        let device_name = Asset::get(channel_identifier.device)
                                            .map(|d| d.name().to_owned())
                                            .unwrap_or_else(|| {
                                                format!(
                                                    "Unknown Device ({})",
                                                    channel_identifier.device
                                                )
                                            });
                                        if ui.label(if let Some(universe) = channel_identifier.universe { format!(
                                            "{device_name} - Universe {universe} - Channel {}:",
                                            channel_identifier.channel
                                        )} else {
                                            format!("{device_name} - Channel {}:", channel_identifier.channel)
                                        }).clicked() {
                                            self.device = Some(channel_identifier.device);
                                            self.universe = channel_identifier.universe;
                                            self.channel = channel_identifier.channel;
                                            self.value = *value;
                                        };
                                        ui.label(RichText::new(value.to_string()).color(Color32::YELLOW));
                                        if ui.button("❌").on_hover_text("Remove this channel overwrite").clicked() {
                                            let mut channel_overwrites = channel_overwrites.clone();
                                            channel_overwrites.0.remove(channel_identifier);
                                            channel_overwrites.set();
                                        }
                                    });
                                }
                            });
                        });
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ChannelOverwrites(
    #[serde(with = "serde_channel_overwrites_map")]
    BTreeMap<ChannelIdentifier, u8>
);

static CHANNEL_OVERWRITES: Lazy<Mutex<ChannelOverwrites>> =
    Lazy::new(|| Mutex::new(ChannelOverwrites::default()));

impl ChannelOverwrites {
    pub fn get() -> ChannelOverwrites {
        CHANNEL_OVERWRITES.lock().clone()
    }

    pub fn set(self) {
        *CHANNEL_OVERWRITES.lock() = self;
    }

    /// Overwrite data for given device/universe tuple.
    /// Removes the entry from the channel overwrites so we know which universes we need to send extra
    pub fn overwrite_data(&mut self, device: AssetId<OutputDevice>, universe: Option<u16>, data: &mut[u8]) {
        self.0.retain(|channel_identifier, value| {
            if channel_identifier.device == device && channel_identifier.universe == universe {
                data[channel_identifier.channel as usize - 1] = *value;
                false
            } else {
                true
            }
        });
    }

    pub fn other_universes(self) -> Vec<(AssetId<OutputDevice>, Option<u16>, [u8; 512])> {
        let mut other_universes = BTreeMap::new();
        for (channel_identifier, value) in self.0.into_iter() {
            other_universes.entry((channel_identifier.device, channel_identifier.universe)).or_insert([0; 512])[channel_identifier.channel as usize - 1] = value;
        }
        other_universes.into_iter().map(|((device, universe), data)| (device, universe, data)).collect()
    }
}

#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Deserialize, serde::Serialize,
)]
pub struct ChannelIdentifier {
    pub device: AssetId<OutputDevice>,
    pub universe: Option<u16>,
    pub channel: u16,
}

mod serde_channel_overwrites_map {
    use super::ChannelIdentifier;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S>(
        map: &BTreeMap<ChannelIdentifier, u8>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<(&ChannelIdentifier, &u8)> = map.iter().collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<ChannelIdentifier, u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<(ChannelIdentifier, u8)> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}