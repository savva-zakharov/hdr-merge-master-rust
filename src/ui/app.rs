//! Main application UI and state management

use eframe::egui;
use std::path::Path;

use crate::config::{Config, GuiSettings, GuiSettingsConfig, OptionalExesAvailable, ExePaths, Profile, FolderEntry};
use crate::scan_folder::ScannedFile;
use crate::ui::{SetupDialog, ProfileManagerDialog, EditProfileDialog, ClearProfilesConfirmDialog};
use crate::ui::{ProfileAction, EditProfileAction, ClearProfilesAction, DialogAction};
use crate::process;

pub struct HdrMergeApp {
    batch_folders: Vec<FolderEntry>,
    selected_index: Option<usize>,
    gui_settings: GuiSettings,
    profiles: Vec<String>,
    progress: f32,
    status_message: String,
    config: Config,
    setup_dialog: SetupDialog,
    profile_manager_dialog: ProfileManagerDialog,
    edit_profile_dialog: EditProfileDialog,
    clear_profiles_confirm_dialog: ClearProfilesConfirmDialog,
}

impl Default for HdrMergeApp {
    fn default() -> Self {
        Self {
            batch_folders: Vec::new(),
            selected_index: None,
            gui_settings: GuiSettings::default(),
            profiles: vec![
                "Default".to_string(),
                "Landscape".to_string(),
                "Portrait".to_string(),
                "Night".to_string(),
            ],
            progress: 0.0,
            status_message: String::new(),
            config: Config {
                _needs_setup: false,
                _optional_exes_available: OptionalExesAvailable::default(),
                exe_paths: ExePaths::default(),
                gui_settings: GuiSettingsConfig::default(),
                pp3_profiles: Vec::new(),
            },
            setup_dialog: SetupDialog::new(),
            profile_manager_dialog: ProfileManagerDialog::new(),
            edit_profile_dialog: EditProfileDialog::new(),
            clear_profiles_confirm_dialog: ClearProfilesConfirmDialog::new(),
        }
    }
}

impl HdrMergeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set light theme
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        // Load configuration
        let config = Config::load("config.json").unwrap_or_else(|_| Config {
            _needs_setup: false,
            _optional_exes_available: OptionalExesAvailable::default(),
            exe_paths: ExePaths::default(),
            gui_settings: GuiSettingsConfig::default(),
            pp3_profiles: Vec::new(),
        });

        // Initialize gui_settings from config
        let gui_settings = GuiSettings {
            threads: config.gui_settings.threads as u32,
            do_recursive: config.gui_settings.do_recursive,
            do_cleanup: config.gui_settings.do_cleanup,
            do_align: config.gui_settings.do_align,
            use_opencv: config.gui_settings.use_opencv,
            use_opencv_merge: config.gui_settings.use_opencv_merge,
            use_opencv_tonemap: config.gui_settings.use_opencv_tonemap,
            tonemap_operator: config.gui_settings.tonemap_operator.clone(),
            tonemap_intensity: config.gui_settings.tonemap_intensity,
            tonemap_contrast: config.gui_settings.tonemap_contrast,
            tonemap_saturation: config.gui_settings.tonemap_saturation,
        };

        // Initialize profiles from config or use defaults
        let profiles = if config.pp3_profiles.is_empty() {
            vec![
                "Default".to_string(),
                "Landscape".to_string(),
                "Portrait".to_string(),
                "Night".to_string(),
            ]
        } else {
            config.pp3_profiles.iter().map(|p| p.name.clone()).collect()
        };

        Self {
            config,
            gui_settings,
            profiles,
            ..Default::default()
        }
    }

    fn format_exif_info(file: &ScannedFile) -> String {
        let mut info_parts = Vec::new();

        if let Some(exp) = &file.exposure_time {
            info_parts.push(format!("Exposure: {}", exp));
        }
        if let Some(fnum) = &file.f_number {
            info_parts.push(format!("FNumber: {}", fnum));
        }
        if let Some(iso) = &file.iso {
            info_parts.push(format!("ISO: {}", iso));
        }

        if info_parts.is_empty() {
            String::new()
        } else {
            info_parts.join(" | ")
        }
    }

    fn open_setup(&mut self) {
        self.setup_dialog.open(&self.config);
    }

    fn handle_setup_action(&mut self, action: DialogAction) {
        match action {
            DialogAction::Save => {
                if let Some(config) = self.setup_dialog.save() {
                    if let Err(e) = config.save("config.json") {
                        self.status_message = format!("Failed to save config: {}", e);
                    } else {
                        self.status_message = "Configuration saved".to_string();

                        // Update main app state with new config
                        self.gui_settings.threads = config.gui_settings.threads as u32;
                        self.gui_settings.do_recursive = config.gui_settings.do_recursive;
                        self.gui_settings.do_cleanup = config.gui_settings.do_cleanup;
                        self.gui_settings.do_align = config.gui_settings.do_align;
                        self.gui_settings.use_opencv = config.gui_settings.use_opencv;

                        // Update profiles from config
                        self.profiles = if config.pp3_profiles.is_empty() {
                            vec![
                                "Default".to_string(),
                                "Landscape".to_string(),
                                "Portrait".to_string(),
                                "Night".to_string(),
                            ]
                        } else {
                            config.pp3_profiles.iter().map(|p| p.name.clone()).collect()
                        };

                        self.config = config;
                    }
                }
            }
            DialogAction::Cancel => {
                self.setup_dialog.cancel();
            }
            DialogAction::None => {}
        }
    }


    fn add_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let path_str = path.to_string_lossy().to_string();

            // Check if already in batch
            if self.batch_folders.iter().any(|f| f.path == path_str) {
                return;
            }

            // Use the scan_folder module to scan the folder
            let scan_result = crate::scan_folder::scan_folder(
                &path,
                &self.config.gui_settings.processed_extensions,
                &self.config.gui_settings.raw_extensions,
            );

            // Determine the primary extension from scanned files
            let extension = scan_result.files
                .first()
                .and_then(|f| Path::new(&f.path).extension())
                .map(|ext| format!(".{}", ext.to_string_lossy().to_lowercase()))
                .unwrap_or_else(|| self.config.gui_settings.processed_extensions.first().cloned().unwrap_or_else(|| ".tiff".to_string()));

            self.batch_folders.push(FolderEntry {
                path: path_str,
                profile: self.profiles.first().cloned().unwrap_or_else(|| "Default".to_string()),
                extension,
                is_raw: scan_result.is_raw,
                align: self.gui_settings.do_align,
                brackets: scan_result.brackets,
                sets: scan_result.sets,
                files: scan_result.files,
            });
        }
    }

    fn open_profile_manager(&mut self) {
        self.profile_manager_dialog.open();
    }

    fn handle_profile_action(&mut self, action: ProfileAction) {
        match action {
            ProfileAction::Add => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PP3 File", &["pp3"])
                    .pick_file()
                {
                    let path_str = path.to_string_lossy().to_string();
                    let name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("New Profile")
                        .to_string();

                    let profile = Profile {
                        name: name.clone(),
                        file_path: path_str.clone(),
                        tag: String::new(),
                    };

                    self.config.pp3_profiles.push(profile);
                    self.profiles.push(name);
                    let _ = self.config.save("config.json");
                }
            }
            ProfileAction::Delete(index) => {
                if index < self.config.pp3_profiles.len() {
                    self.config.pp3_profiles.remove(index);
                }
                if index < self.profiles.len() {
                    self.profiles.remove(index);
                }
                let _ = self.config.save("config.json");
            }
            ProfileAction::Edit(index) => {
                let profile = self.config.pp3_profiles.get(index);
                let (name, path, tag) = if let Some(p) = profile {
                    (p.name.clone(), p.file_path.clone(), p.tag.clone())
                } else {
                    (self.profiles.get(index).cloned().unwrap_or_default(), String::new(), String::new())
                };
                self.edit_profile_dialog.open(index, &name, &path, &tag);
            }
            ProfileAction::ClearAll => {
                self.clear_profiles_confirm_dialog.open();
            }
            ProfileAction::Close => {
                self.profile_manager_dialog.close();
            }
            ProfileAction::None => {}
        }
    }

    fn handle_edit_profile_action(&mut self, action: EditProfileAction) {
        match action {
            EditProfileAction::Save => {
                let (index, name, path, tag) = self.edit_profile_dialog.get_result();
                if let Some(idx) = index {
                    if idx < self.config.pp3_profiles.len() {
                        self.config.pp3_profiles[idx].name = name.clone();
                        self.config.pp3_profiles[idx].file_path = path.clone();
                        self.config.pp3_profiles[idx].tag = tag;
                    }
                    if idx < self.profiles.len() {
                        self.profiles[idx] = name;
                    }
                    let _ = self.config.save("config.json");
                }
                self.edit_profile_dialog.close();
            }
            EditProfileAction::Cancel => {
                self.edit_profile_dialog.close();
            }
            EditProfileAction::None => {}
        }
    }

    fn handle_clear_profiles_action(&mut self, action: ClearProfilesAction) {
        match action {
            ClearProfilesAction::Confirm => {
                self.config.pp3_profiles.clear();
                self.profiles = vec![
                    "Default".to_string(),
                    "Landscape".to_string(),
                    "Portrait".to_string(),
                    "Night".to_string(),
                ];
                let _ = self.config.save("config.json");
                self.clear_profiles_confirm_dialog.close();
            }
            ClearProfilesAction::Cancel => {
                self.clear_profiles_confirm_dialog.close();
            }
            ClearProfilesAction::None => {}
        }
    }


    fn remove_selected(&mut self) {
        if let Some(index) = self.selected_index {
            if index < self.batch_folders.len() {
                self.batch_folders.remove(index);
                self.selected_index = None;
            }
        }
    }

    fn clear_all(&mut self) {
        self.batch_folders.clear();
        self.selected_index = None;
    }

    fn export_batch(&mut self) {
        if self.batch_folders.is_empty() {
            self.status_message = "No folders to export".to_string();
            return;
        }

        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("batch_export.json")
            .save_file()
        {
            let json = serde_json::to_string_pretty(&self.batch_folders);
            match json {
                Ok(content) => {
                    if let Err(e) = std::fs::write(path, content) {
                        self.status_message = format!("Export failed: {}", e);
                    } else {
                        self.status_message = "Export successful".to_string();
                    }
                }
                Err(e) => self.status_message = format!("Serialize failed: {}", e),
            }
        }
    }

    fn import_batch(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<Vec<FolderEntry>>(&content) {
                        Ok(folders) => {
                            self.batch_folders.extend(folders);
                            self.status_message = "Import successful".to_string();
                        }
                        Err(e) => self.status_message = format!("Parse failed: {}", e),
                    }
                }
                Err(e) => self.status_message = format!("Read failed: {}", e),
            }
        }
    }

    fn execute(&mut self) {
        if self.batch_folders.is_empty() {
            self.status_message = "Please add folders first".to_string();
            return;
        }

        let total_folders = self.batch_folders.len();
        let mut processed = 0;
        let mut errors = Vec::new();

        for (_idx, folder) in self.batch_folders.iter().enumerate() {
            match process::process_folder(folder, &self.config, &self.gui_settings) {
                Ok(msg) => {
                    self.status_message = msg;
                    processed += 1;
                }
                Err(err) => {
                    errors.push(format!("{}: {}", folder.path, err));
                }
            }

            // Update progress
            self.progress = (processed as f32) / (total_folders as f32);
        }

        // Final status message
        if errors.is_empty() {
            self.status_message = format!("Successfully processed {} folders", processed);
            self.progress = 1.0;
        } else {
            self.status_message = format!(
                "Completed with errors: {} of {} folders processed. Errors: {}",
                processed,
                total_folders,
                errors.join("; ")
            );
        }
    }
}

impl eframe::App for HdrMergeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle setup dialog
        let setup_action = self.setup_dialog.render(ctx);
        self.handle_setup_action(setup_action);

        // Handle profile manager dialog
        let profile_action = self.profile_manager_dialog.render(ctx, &self.profiles, &self.config.pp3_profiles);
        self.handle_profile_action(profile_action);

        // Handle edit profile dialog
        let edit_action = self.edit_profile_dialog.render(ctx);
        self.handle_edit_profile_action(edit_action);

        // Handle clear profiles confirmation dialog
        let clear_action = self.clear_profiles_confirm_dialog.render(ctx);
        self.handle_clear_profiles_action(clear_action);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                // ========== Batch Folders Section ==========
                ui.horizontal(|ui| {
                    ui.label("Input Folders:");

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            // Folder table
                            egui::ScrollArea::vertical()
                                .max_height(800.0)
                                .max_width(800.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("folder_grid")
                                        .num_columns(7)
                                        .spacing([10.0, 4.0])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            ui.label("Folder");
                                            ui.label("Profile");
                                            ui.label("Ext");
                                            ui.label("RAW");
                                            ui.label("Align");
                                            ui.label("Brackets");
                                            ui.label("Sets");
                                            ui.end_row();

                                            for (i, folder) in self.batch_folders.iter().enumerate() {
                                                let is_selected = self.selected_index == Some(i);
                                                if is_selected {
                                                    ui.scope(|ui| {
                                                        ui.visuals_mut().selection.bg_fill = egui::Color32::from_rgb(200, 200, 255);
                                                        let _ = ui.selectable_label(true, &folder.path);
                                                    });
                                                } else {
                                                    if ui.selectable_label(is_selected, &folder.path).clicked() {
                                                        self.selected_index = Some(i);
                                                    }
                                                }
                                                ui.label(&folder.profile);
                                                ui.label(&folder.extension);
                                                ui.label(if folder.is_raw { "Yes" } else { "No" });
                                                ui.label(if folder.align { "Yes" } else { "No" });
                                                ui.label(folder.brackets.to_string());
                                                ui.label(format!("{} files", folder.files.len()));
                                                ui.end_row();
                                            }
                                        });
                                });

                            // Buttons
                            ui.vertical(|ui| {
                                if ui.button("Add").clicked() {
                                    self.add_folder();
                                }
                                if ui.button("Remove").clicked() {
                                    self.remove_selected();
                                }
                                if ui.button("Clear All").clicked() {
                                    self.clear_all();
                                }
                                ui.separator();
                                if ui.button("Export").clicked() {
                                    self.export_batch();
                                }
                                if ui.button("Import").clicked() {
                                    self.import_batch();
                                }
                                ui.checkbox(&mut self.gui_settings.do_recursive, "Recursive");
                            });
                        });
                    });
                });

                // Show files for selected folder
                if let Some(index) = self.selected_index {
                    if let Some(folder) = self.batch_folders.get(index) {
                        ui.separator();
                        ui.label(format!("Files in: {} ({} files, {} brackets, {} sets)",
                            folder.path, folder.files.len(), folder.brackets, folder.sets));

                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .show(ui, |ui| {
                                egui::Grid::new("files_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 4.0])
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label("File Path");
                                        ui.label("EXIF Info");
                                        ui.end_row();

                                        for file in &folder.files {
                                            ui.label(&file.path);
                                            let exif_info = Self::format_exif_info(file);
                                            ui.label(if exif_info.is_empty() { "No EXIF" } else { &exif_info });
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                }

                ui.separator();

                // ========== Profile Selection Section ==========
                ui.horizontal(|ui| {
                    ui.label("PP3 Profile:");

                    egui::ComboBox::from_label("")
                        .selected_text(
                            self.selected_index
                                .and_then(|i| self.batch_folders.get(i))
                                .map(|f| f.profile.clone())
                                .unwrap_or_else(|| "Select...".to_string())
                        )
                        .show_ui(ui, |ui| {
                            for profile in &self.profiles {
                                ui.selectable_value(&mut self.selected_index, None, profile);
                            }
                        });

                    ui.checkbox(&mut self.gui_settings.do_align,
                        if self.gui_settings.use_opencv { "Align (OpenCV)" } else { "Align (Hugin)" }
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Manage Profiles...").clicked() {
                            self.open_profile_manager();
                        }
                    });
                });

                ui.separator();

                // ========== Options Section ==========
                ui.horizontal(|ui| {
                    ui.label("Threads:");
                    ui.add(egui::DragValue::new(&mut self.gui_settings.threads)
                        .range(1..=self.config.gui_settings.threads as u32 * 2));

                    ui.checkbox(&mut self.gui_settings.do_cleanup, "Cleanup temporary files");
                    ui.checkbox(&mut self.gui_settings.do_recursive, "Recursive");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Create HDRs").clicked() {
                            self.execute();
                        }
                        if ui.button("Setup").clicked() {
                            self.open_setup();
                        }
                    });
                });

                ui.separator();

                // ========== Progress Bar Section ==========
                ui.horizontal(|ui| {
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                });

                // Status message
                if !self.status_message.is_empty() {
                    ui.label(&self.status_message);
                }
            });
        });
    }
}
