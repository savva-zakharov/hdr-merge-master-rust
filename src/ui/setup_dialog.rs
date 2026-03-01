//! Setup dialog for configuring application settings

use crate::config::{Config};
use eframe::egui;

pub struct SetupDialog {
    pub show: bool,
    pub config: Option<Config>,
}

impl SetupDialog {
    pub fn new() -> Self {
        Self {
            show: false,
            config: None,
        }
    }

    pub fn open(&mut self, config: &Config) {
        self.config = Some(config.clone());
        self.show = true;
    }

    pub fn save(&mut self) -> Option<Config> {
        self.config.take()
    }

    pub fn cancel(&mut self) {
        self.config = None;
        self.show = false;
    }

    pub fn render(&mut self, ctx: &egui::Context) -> DialogAction {
        if !self.show {
            return DialogAction::None;
        }

        let mut action = DialogAction::None;

        egui::Window::new("Setup")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 400.0])
            .show(ctx, |ui| {
                if let Some(config) = &mut self.config {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                            // ========== GUI Settings Section ==========
                            ui.group(|ui| {
                                ui.label("GUI Settings");
                                ui.separator();

                                ui.horizontal(|ui| {
                                    ui.label("Threads:");
                                    ui.add(egui::DragValue::new(&mut config.gui_settings.threads).range(1..=9999));
                                });

                                ui.checkbox(&mut config.gui_settings.do_recursive, "Recursive Processing");
                                ui.checkbox(&mut config.gui_settings.do_cleanup, "Cleanup Temporary Files");
                                ui.checkbox(&mut config.gui_settings.do_align, "Do Align");
                                ui.checkbox(&mut config.gui_settings.use_opencv_align, "Use OpenCV (AlignMTB)");
                                ui.checkbox(&mut config.gui_settings.use_opencv_merge, "Use OpenCV Merge (Debevec)");
                                ui.checkbox(&mut config.gui_settings.use_opencv_tonemap, "Use OpenCV Tone Mapping");

                                ui.horizontal(|ui| {
                                    ui.label("Recursive Max Depth:");
                                    ui.add(egui::DragValue::new(&mut config.gui_settings.recursive_max_depth).range(0..=10));
                                });

                                ui.separator();
                                ui.label("Tone Mapping Operator:");
                                ui.horizontal(|ui| {
                                    ui.radio_value(&mut config.gui_settings.tonemap_operator, "Reinhard".to_string(), "Reinhard");
                                    ui.radio_value(&mut config.gui_settings.tonemap_operator, "Drago".to_string(), "Drago");
                                    ui.radio_value(&mut config.gui_settings.tonemap_operator, "Durand".to_string(), "Durand");
                                    ui.radio_value(&mut config.gui_settings.tonemap_operator, "Mantiuk".to_string(), "Mantiuk");
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Intensity:");
                                    ui.add(egui::DragValue::new(&mut config.gui_settings.tonemap_intensity).range(0.0..=1.0).speed(0.1));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Contrast:");
                                    ui.add(egui::DragValue::new(&mut config.gui_settings.tonemap_contrast).range(0.0..=1.0).speed(0.1));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Saturation:");
                                    ui.add(egui::DragValue::new(&mut config.gui_settings.tonemap_saturation).range(0.0..=1.0).speed(0.1));
                                });

                                ui.separator();
                                ui.label("Processed Extensions (comma-separated):");
                                let mut processed_exts_str = config.gui_settings.processed_extensions.join(",");
                                if ui.add(egui::TextEdit::singleline(&mut processed_exts_str).desired_width(300.0)).changed() {
                                    config.gui_settings.processed_extensions = processed_exts_str
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                }

                                ui.separator();
                                ui.label("Raw Extensions (comma-separated):");
                                let mut raw_exts_str = config.gui_settings.raw_extensions.join(",");
                                if ui.add(egui::TextEdit::singleline(&mut raw_exts_str).desired_width(300.0)).changed() {
                                    config.gui_settings.raw_extensions = raw_exts_str
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                }

                                ui.separator();
                                ui.label("Recursive Ignore Folders (comma-separated):");
                                let mut ignore_folders_str = config.gui_settings.recursive_ignore_folders.join(",");
                                if ui.add(egui::TextEdit::singleline(&mut ignore_folders_str).desired_width(300.0)).changed() {
                                    config.gui_settings.recursive_ignore_folders = ignore_folders_str
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                }
                            });

                            ui.separator();

                            // ========== Executable Paths Section ==========
                            ui.group(|ui| {
                                ui.label("Executable Paths");
                                ui.separator();

                                ui.horizontal(|ui| {
                                    ui.label("Align Image Stack:");
                                    ui.add(egui::TextEdit::singleline(&mut config.exe_paths.align_image_stack_exe).desired_width(300.0));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Blender:");
                                    ui.add(egui::TextEdit::singleline(&mut config.exe_paths.blender_exe).desired_width(300.0));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Luminance CLI:");
                                    ui.add(egui::TextEdit::singleline(&mut config.exe_paths.luminance_cli_exe).desired_width(300.0));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Rawtherapee CLI:");
                                    ui.add(egui::TextEdit::singleline(&mut config.exe_paths.rawtherapee_cli_exe).desired_width(300.0));
                                });
                            });
                        });
                    });

                    // Bottom buttons
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Save").clicked() {
                                action = DialogAction::Save;
                            }
                            if ui.button("Cancel").clicked() {
                                action = DialogAction::Cancel;
                            }
                        });
                    });
                }
            });

        action
    }
}

pub enum DialogAction {
    Save,
    Cancel,
    None,
}
