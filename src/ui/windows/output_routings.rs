use crate::{
    app::svg::universes, extract_output::ExtractOutput, storage::Asset, ui::ChangeButton,
    viewport_builder::default_viewport_builder,
};
use egui::{ComboBox, Context, Id, Layout, RichText, Vec2, ViewportId};

#[derive(Default)]
pub struct OutputRoutingsWindow {
    open: bool,
}

impl OutputRoutingsWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("output routings window")),
            default_viewport_builder()
                .with_title("Gled: Output Routings")
                .with_inner_size(Vec2::new(300.0, 500.0))
                .with_min_inner_size(Vec2::new(300.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("output_scroll")
                        .show(ui, |ui| {
                            let extract_output = ExtractOutput::get();
                            let mut routings = extract_output.routings.lock();

                            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                                let universes = universes();
                                extract_output
                                    .universes
                                    .lock()
                                    .retain(|universe| universes.contains(universe));

                                if universes.is_empty() {
                                    ui.label(RichText::new(
                                    "No universes available. You need to load a svg file first!",
                                ));
                                }

                                for universe in universes {
                                    ui.label(
                                        RichText::new(format!("Universe: {universe}")).heading(),
                                    );

                                    ui.horizontal(|ui| {
                                        let output_routing =
                                            routings.universe_output_routing(universe);
                                        output_routing.device.change_button(ui);

                                        if let Some(device) =
                                            output_routing.device.and_then(Asset::get)
                                        {
                                            let universes = device.data.universes();
                                            if !universes.is_empty() {
                                                ComboBox::new(format!("{universe}_universe"), "")
                                                    .selected_text(match output_routing.universe {
                                                        None => "No universe selected".to_string(),
                                                        Some(universe) => universe.to_string(),
                                                    })
                                                    .width(150.0)
                                                    .show_ui(ui, |ui| {
                                                        for universe in universes {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut output_routing
                                                                        .universe
                                                                        .unwrap_or_default(),
                                                                    *universe,
                                                                    universe.to_string(),
                                                                )
                                                                .changed()
                                                            {
                                                                output_routing.universe =
                                                                    Some(*universe);
                                                            }
                                                        }
                                                    });
                                            }
                                        }
                                    });

                                    ui.separator();
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
