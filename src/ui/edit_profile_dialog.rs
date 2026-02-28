//! Edit profile dialog for editing individual profile details

use eframe::egui;

pub struct EditProfileDialog {
    pub show: bool,
    pub editing_index: Option<usize>,
    pub name: String,
    pub file_path: String,
    pub tag: String,
}

pub enum EditProfileAction {
    Save,
    Cancel,
    None,
}

impl EditProfileDialog {
    pub fn new() -> Self {
        Self {
            show: false,
            editing_index: None,
            name: String::new(),
            file_path: String::new(),
            tag: String::new(),
        }
    }

    pub fn open(&mut self, index: usize, name: &str, file_path: &str, tag: &str) {
        self.editing_index = Some(index);
        self.name = name.to_string();
        self.file_path = file_path.to_string();
        self.tag = tag.to_string();
        self.show = true;
    }

    pub fn close(&mut self) {
        self.editing_index = None;
        self.show = false;
    }

    pub fn get_result(&self) -> (Option<usize>, String, String, String) {
        (
            self.editing_index,
            self.name.clone(),
            self.file_path.clone(),
            self.tag.clone(),
        )
    }

    pub fn render(&mut self, ctx: &egui::Context) -> EditProfileAction {
        if !self.show {
            return EditProfileAction::None;
        }

        let mut action = EditProfileAction::None;

        egui::Window::new("Edit Profile")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 250.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(250.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("File Path:");
                        ui.add(egui::TextEdit::singleline(&mut self.file_path).desired_width(250.0));
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PP3 File", &["pp3"])
                                .pick_file()
                            {
                                self.file_path = path.to_string_lossy().to_string();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tag:");
                        ui.add(egui::TextEdit::singleline(&mut self.tag).desired_width(250.0));
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Save").clicked() {
                                action = EditProfileAction::Save;
                            }
                            if ui.button("Cancel").clicked() {
                                action = EditProfileAction::Cancel;
                            }
                        });
                    });
                });
            });

        action
    }
}
