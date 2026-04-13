use super::*;

impl<T: AssetTrait> AssetTree<T> {
    /// Returns true if the selection has changed
    pub fn show(&mut self, ui: &mut Ui, id: Id, collections: &mut Collections) -> bool {
        let mut selection_changed = false;
        ScrollArea::vertical()
            .scroll_bar_visibility(AlwaysVisible)
            .id_salt(format!("scroll: {id:?}"))
            .show(ui, |ui| {
                let entries = self.load(collections);
                let mut tree_ids = vec![];

                if !self.only_asset_selection {
                    ui.horizontal(|ui| {
                        let new_dir_name = vec!["New Folder".to_string()];
                        if ui
                            .add_enabled(
                                !self.empty_dirs.contains(&new_dir_name)
                                    && !entries.iter().any(|child| {
                                        let TreeEntry::Dir(dir, ..) = child else {
                                            return false;
                                        };
                                        dir == &new_dir_name
                                    }),
                                Button::new("+Folder"),
                            )
                            .clicked()
                        {
                            self.empty_dirs.push(new_dir_name);
                        }
                        if ui
                            .add_enabled(
                                !self.empty_dirs.is_empty(),
                                Button::new("Clear Empty Folders"),
                            )
                            .clicked()
                        {
                            if matches!(self.selection, TreeSelection::Dir { .. }) {
                                self.selection = TreeSelection::None;
                            }
                            self.empty_dirs.clear();
                        }
                    });
                }

                let actions = TreeView::new(id)
                    .show(ui, |builder| {
                        for entry in entries.iter() {
                            entry.build(
                                &mut self.empty_dirs,
                                &mut tree_ids,
                                &mut Some(builder),
                                self.only_asset_selection,
                                collections,
                            );
                        }
                    })
                    .1;

                for action in actions {
                    match action {
                        Action::SetSelected(index) => {
                            self.selection = index
                                .first()
                                .copied()
                                .map(|index| tree_ids.remove(index))
                                .and_then(|id| match id {
                                    TreeId::File(id) => Asset::get(id, collections).map(|asset| {
                                        TreeSelection::Asset(Arc::unwrap_or_clone(asset))
                                    }),
                                    TreeId::Dir(dir) => Some(TreeSelection::Dir {
                                        current: dir.clone(),
                                        new: dir,
                                    }),
                                })
                                .unwrap_or_default();
                            selection_changed = true;
                            // we do not want keyboard focus so that keyboard bindings still work
                            ui.memory_mut(|memory|memory.stop_text_input());
                        }
                        Action::Move(DragAndDrop { source, target, .. }) => {
                            for source in source {
                                let mut target = target;
                                if source < target {
                                    target -= 1;
                                }

                                let source = tree_ids.remove(source);
                                let target = tree_ids.remove(target);

                                if let (TreeId::File(source), TreeId::Dir(target)) =
                                    (source, target)
                                {
                                    let target = target.clone();
                                    if let Some(asset) = Asset::get(source, collections) {
                                        let mut asset = Arc::unwrap_or_clone(asset);
                                        asset.path = target
                                            .into_iter()
                                            .chain(std::iter::once(
                                                asset.path.last().unwrap().clone(),
                                            ))
                                            .collect();
                                        asset.save(collections);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });

        selection_changed
    }

    pub fn selected(&mut self) -> &mut TreeSelection<T> {
        &mut self.selection
    }

    pub fn common_settings(
        &mut self,
        ui: &mut Ui,
        dirty: &mut bool,
        collections: &mut Collections,
    ) -> bool {
        let mut changed = false;

        egui::Frame::NONE
            .inner_margin(Margin::from(6.0))
            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
            .show(ui, |ui| {
                match &mut self.selection {
                    TreeSelection::Asset(asset) => {
                        ui.label("Name:");
                        let mut name = asset.name().to_string();
                        let res = ui
                            .vertical_centered_justified(|ui| ui.text_edit_singleline(&mut name))
                            .inner;
                        if res.changed() {
                            *dirty = true;
                            asset.path.pop();
                            asset.path.push(name);
                        }
                    }
                    TreeSelection::Dir { .. } => {
                        self.show_folder_editor(ui, collections);
                        return;
                    }
                    _ => {
                        ui.label("Please select an item from the tree");
                        return;
                    }
                }

                ui.add_space(4.0);

                let width = ui.available_width() - 40.0;
                ui.horizontal(|ui| {
                    egui::Frame::NONE
                        .inner_margin(Margin::from(3.0))
                        .show(ui, |ui| {
                            ui.set_max_width(width / 4.0);
                            ui.vertical_centered_justified(|ui| {
                                if ui
                                    .add(Button::new(format!(
                                        "Save{}",
                                        if *dirty { "*" } else { "" }
                                    )))
                                    .on_hover_ui(|ui| {
                                        ui.label("Save to disk");
                                    })
                                    .clicked()
                                {
                                    *dirty = false;

                                    if let TreeSelection::Asset(asset) = &self.selection {
                                        asset.clone().save(collections);
                                    }

                                    changed = true;
                                }
                            });
                        });
                    egui::Frame::NONE
                        .inner_margin(Margin::from(3.0))
                        .show(ui, |ui| {
                            ui.set_max_width(width / 4.0 - 6.0);
                            ui.vertical_centered_justified(|ui| {
                                if ui
                                    .add(
                                        Button::new("Save Copy")
                                            .wrap_mode(egui::TextWrapMode::Extend),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label("Save copy to disk");
                                    })
                                    .clicked()
                                {
                                    *dirty = false;

                                    if let TreeSelection::Asset(asset) = &self.selection {
                                        asset.copy().save(collections);
                                    }

                                    changed = true;
                                }
                            });
                        });

                    egui::Frame::NONE
                        .inner_margin(Margin::from(3.0))
                        .show(ui, |ui| {
                            ui.set_max_width(width / 4.0);
                            ui.vertical_centered_justified(|ui| {
                                if ui
                                    .add(
                                        Button::new(format!(
                                            "Reset{}",
                                            if *dirty { "*" } else { "" }
                                        ))
                                        .fill(Color32::DARK_RED),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label("Reset to state on disk");
                                    })
                                    .clicked()
                                {
                                    *dirty = false;
                                    if let TreeSelection::Asset(asset) = &mut self.selection {
                                        *asset = Arc::unwrap_or_clone(
                                            Asset::get(asset.id, collections).unwrap_or_default(),
                                        );
                                    }

                                    changed = true;
                                }
                            });
                        });

                    egui::Frame::NONE
                        .inner_margin(Margin::from(3.0))
                        .show(ui, |ui| {
                            ui.set_max_width(width / 4.0);
                            ui.vertical_centered_justified(|ui| {
                                if ui
                                    .add(Button::new("Delete").fill(Color32::DARK_RED))
                                    .clicked()
                                {
                                    if let TreeSelection::Asset(asset) = &self.selection {
                                        asset.delete(collections);
                                    }
                                    self.selection = TreeSelection::None;

                                    changed = true
                                }
                            });
                        });
                });
            });

        changed
    }

    fn show_folder_editor(&mut self, ui: &mut Ui, collections: &mut Collections) {
        let TreeSelection::Dir { current, new } = &mut self.selection else {
            return;
        };

        ui.label("Name:");
        let mut name = new.last().cloned().unwrap_or_default();
        let res = ui
            .vertical_centered_justified(|ui| ui.text_edit_singleline(&mut name))
            .inner;
        if res.changed() {
            self.folder_dirty = true;
            new.pop();
            new.push(name);
        }

        ui.add_enabled_ui(self.folder_dirty, |ui| {
            ui.vertical_centered_justified(|ui| {
                if ui
                    .button("Save")
                    .on_hover_ui(|ui| {
                        ui.label("Save all assets in this folder with new name to disk");
                    })
                    .clicked()
                {
                    self.folder_dirty = false;
                    let assets = Asset::<T>::all(collections);
                    for asset in assets {
                        if &asset.dir() == current {
                            let mut asset = Arc::unwrap_or_clone(asset);
                            asset.change_dir(new);
                            asset.save(collections);
                        }
                    }
                    for empty_dir in self.empty_dirs.iter_mut() {
                        if empty_dir.len() >= current.len()
                            && empty_dir[..current.len()] == current[..]
                        {
                            empty_dir[..current.len()].clone_from_slice(&new[..]);
                        }
                    }
                    *current = new.clone();
                }
            });
        });
    }

    pub fn show_asset_selection(
        ui: &mut Ui,
        id: Id,
        collections: &mut Collections,
    ) -> Option<AssetId<T>> {
        let mut tree = AssetTree {
            only_asset_selection: true,
            ..Default::default()
        };
        tree.show(ui, id, collections);
        match tree.selection {
            TreeSelection::Asset(asset) => Some(asset.id),
            _ => None,
        }
    }
}
