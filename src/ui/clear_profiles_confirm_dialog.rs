//! Confirmation dialog for clearing all profiles

use eframe::egui;

pub struct ClearProfilesConfirmDialog {
    pub show: bool,
}

pub enum ClearProfilesAction {
    Confirm,
    Cancel,
    None,
}

impl ClearProfilesConfirmDialog {
    pub fn new() -> Self {
        Self { show: false }
    }

    pub fn open(&mut self) {
        self.show = true;
    }

    pub fn close(&mut self) {
        self.show = false;
    }

    pub fn render(&mut self, ctx: &egui::Context) -> ClearProfilesAction {
        if !self.show {
            return ClearProfilesAction::None;
        }

        let mut action = ClearProfilesAction::None;

        egui::Window::new("Confirm Clear All")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Are you sure you want to delete all profiles?");
                ui.label("This action cannot be undone.");

                ui.separator();
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Yes, Clear All").clicked() {
                            action = ClearProfilesAction::Confirm;
                        }
                        if ui.button("Cancel").clicked() {
                            action = ClearProfilesAction::Cancel;
                        }
                    });
                });
            });

        action
    }
}
