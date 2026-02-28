//! Profile manager dialog for managing PP3 profiles

use crate::config::Profile;
use eframe::egui;

pub struct ProfileManagerDialog {
    pub show: bool,
}

pub enum ProfileAction {
    Add,
    Delete(usize),
    Edit(usize),
    ClearAll,
    Close,
    None,
}

impl ProfileManagerDialog {
    pub fn new() -> Self {
        Self { show: false }
    }

    pub fn open(&mut self) {
        self.show = true;
    }

    pub fn close(&mut self) {
        self.show = false;
    }

    pub fn render(&mut self, ctx: &egui::Context, profiles: &[String], config_profiles: &[Profile]) -> ProfileAction {
        if !self.show {
            return ProfileAction::None;
        }

        let mut action = ProfileAction::None;

        egui::Window::new("Manage Profiles")
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Profile list
                    ui.label("Saved Profiles:");

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            egui::Grid::new("profiles_grid")
                                .num_columns(4)
                                .spacing([10.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Name");
                                    ui.label("File Path");
                                    ui.label("Tag");
                                    ui.label("Actions");
                                    ui.end_row();

                                    let mut delete_idx = None;
                                    let mut edit_idx = None;

                                    for (i, name) in profiles.iter().enumerate() {
                                        let profile = config_profiles.get(i);
                                        let path = profile.map(|p| p.file_path.clone()).unwrap_or_default();
                                        let tag = profile.map(|p| p.tag.clone()).unwrap_or_default();

                                        ui.label(name);
                                        ui.label(&path);
                                        ui.label(&tag);

                                        ui.horizontal(|ui| {
                                            if ui.button("Edit").clicked() {
                                                edit_idx = Some(i);
                                            }
                                            if ui.button("Delete").clicked() {
                                                delete_idx = Some(i);
                                            }
                                        });
                                        ui.end_row();
                                    }

                                    if let Some(idx) = delete_idx {
                                        action = ProfileAction::Delete(idx);
                                    }
                                    if let Some(idx) = edit_idx {
                                        action = ProfileAction::Edit(idx);
                                    }
                                });
                        });

                    // Buttons
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            action = ProfileAction::Add;
                        }
                        if ui.button("Clear All").clicked() {
                            action = ProfileAction::ClearAll;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                action = ProfileAction::Close;
                            }
                        });
                    });
                });
            });

        action
    }
}
