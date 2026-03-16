//! Main application UI and state management

use iced::widget::{button, checkbox, container, pick_list, progress_bar, rule, scrollable, text, Column, Row, space, row};
use iced::Length::Fill;
use iced::{keyboard, Alignment, Element, Length, Subscription, Task, Theme};
use std::path::Path;

use crate::config::{
    Config, ExePaths, FolderEntry, GuiSettings, GuiSettingsConfig, OptionalExesAvailable, Profile,
};
use crate::process;
use crate::scan_folder::ScannedFile;
use crate::ui::{ClearProfilesConfirmDialog, EditProfileDialog, ProfileManagerDialog, SetupDialog};
use crate::ui::clear_profiles_confirm_dialog::ClearProfilesMessage;
use crate::ui::edit_profile_dialog::EditProfileMessage;
use crate::ui::profile_manager_dialog::ProfileMessage;
use crate::ui::setup_dialog::DialogMessage;

pub struct HdrMergeApp {
    batch_folders: Vec<FolderEntry>,
    selected_index: Option<usize>,
    gui_settings: GuiSettings,
    profiles: Vec<String>,
    progress: f32,
    status_message: String,
    config: Config,
    uiscale: f32,
    theme: Option<Theme>,
    setup_dialog: SetupDialog,
    profile_manager_dialog: ProfileManagerDialog,
    edit_profile_dialog: EditProfileDialog,
    clear_profiles_confirm_dialog: ClearProfilesConfirmDialog,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // Main app actions
    AddFolder,
    RemoveSelected,
    ClearAll,
    ExportBatch,
    ImportBatch,
    Execute,
    SelectFolder(usize),
    SelectProfile(String),
    SelectFolderProfile(usize, String),
    ToggleRecursive(bool),
    ToggleAlign(bool),
    ToggleFolderAlign(usize, bool),
    ToggleCleanup(bool),
    ThreadsChanged(String),

    // Setup dialog
    OpenSetup,
    SetupDialogMsg(DialogMessage),

    // Profile manager
    OpenProfileManager,
    ProfileManagerMsg(ProfileMessage),

    // Edit profile
    EditProfileMsg(EditProfileMessage),

    // Clear profiles
    ClearProfilesMsg(ClearProfilesMessage),

    // File dialog results
    FolderSelected(Option<String>),
    BatchExportSelected(Option<String>),
    BatchImportSelected(Option<String>),
    ProfileFileSelected(Option<String>),

    // UI scaling
    FontSizeIncreased,
    FontSizeDecreased,

    // Theme
    ThemeChanged(Theme),
    PreviousTheme,
    NextTheme,
    ClearTheme,
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
            uiscale: 1.0,
            theme: None,
            setup_dialog: SetupDialog::new(),
            profile_manager_dialog: ProfileManagerDialog::new(),
            edit_profile_dialog: EditProfileDialog::new(),
            clear_profiles_confirm_dialog: ClearProfilesConfirmDialog::new(),
        }
    }
}

impl HdrMergeApp {
    pub fn new() -> (Self, Task<Message>) {
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
            use_opencv_align: config.gui_settings.use_opencv_align,
            use_align_image_stack: config.gui_settings.use_align_image_stack,
            use_blender_merge: config.gui_settings.use_blender_merge,
            use_opencv_debevec: config.gui_settings.use_opencv_debevec,
            use_opencv_merge_robertson: config.gui_settings.use_opencv_merge_robertson,
            use_rust_merge: config.gui_settings.use_rust_merge,
            use_opencv_tonemap: config.gui_settings.use_opencv_tonemap,
            use_luminance_tonemap: config.gui_settings.use_luminance_tonemap,
            rust_merge_debug_export: config.gui_settings.rust_merge_debug_export,
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

        // Initialize theme from config
        let theme = if config.gui_settings.theme_name.is_empty() {
            None
        } else {
            Theme::ALL.iter().find(|t| format!("{:?}", t) == config.gui_settings.theme_name).cloned()
        };

        let app = Self {
            config: config.clone(),
            gui_settings,
            profiles,
            uiscale: config.gui_settings.uiscale,
            theme,
            ..Default::default()
        };

        (app, Task::none())
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

    fn add_folder(&mut self) -> Task<Message> {
        Task::perform(
            async {
                let file_dialog = rfd::AsyncFileDialog::new();
                file_dialog.pick_folder().await
            },
            |folder| {
                folder.map(|f| f.path().to_string_lossy().to_string())
            },
        ).map(Message::FolderSelected)
    }

    fn handle_folder_selected(&mut self, path_opt: Option<String>) {
        if let Some(path_str) = path_opt {
            // Check if already in batch
            if self.batch_folders.iter().any(|f| f.path == path_str) {
                return;
            }

            let path = Path::new(&path_str);
            
            // Use the scan_folder module to scan the folder
            let scan_result = crate::scan_folder::scan_folder(
                path,
                &self.config.gui_settings.processed_extensions,
                &self.config.gui_settings.raw_extensions,
            );

            // Determine the primary extension from scanned files
            let extension = scan_result
                .files
                .first()
                .and_then(|f| Path::new(&f.path).extension())
                .map(|ext| format!(".{}", ext.to_string_lossy().to_lowercase()))
                .unwrap_or_else(|| {
                    self.config
                        .gui_settings
                        .processed_extensions
                        .first()
                        .cloned()
                        .unwrap_or_else(|| ".tiff".to_string())
                });

            self.batch_folders.push(FolderEntry {
                path: path_str,
                profile: self
                    .profiles
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Default".to_string()),
                extension,
                is_raw: scan_result.is_raw,
                align: self.gui_settings.do_align,
                brackets: scan_result.brackets,
                sets: scan_result.sets,
                files: scan_result.files,
            });
        }
    }

    fn export_batch(&mut self) -> Task<Message> {
        Task::perform(
            async {
                let file_dialog = rfd::AsyncFileDialog::new()
                    .set_file_name("batch_export.json");
                file_dialog.save_file().await
            },
            |file| {
                file.map(|f| f.path().to_string_lossy().to_string())
            },
        ).map(Message::BatchExportSelected)
    }

    fn handle_batch_export(&mut self, path_opt: Option<String>) {
        if let Some(path_str) = path_opt {
            let json = serde_json::to_string_pretty(&self.batch_folders);
            match json {
                Ok(content) => {
                    if let Err(e) = std::fs::write(&path_str, content) {
                        self.status_message = format!("Export failed: {}", e);
                    } else {
                        self.status_message = "Export successful".to_string();
                    }
                }
                Err(e) => self.status_message = format!("Serialize failed: {}", e),
            }
        }
    }

    fn import_batch(&mut self) -> Task<Message> {
        Task::perform(
            async {
                let file_dialog = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"]);
                file_dialog.pick_file().await
            },
            |file| {
                file.map(|f| f.path().to_string_lossy().to_string())
            },
        ).map(Message::BatchImportSelected)
    }

    fn handle_batch_import(&mut self, path_opt: Option<String>) {
        if let Some(path_str) = path_opt {
            match std::fs::read_to_string(&path_str) {
                Ok(content) => match serde_json::from_str::<Vec<FolderEntry>>(&content) {
                    Ok(folders) => {
                        self.batch_folders.extend(folders);
                        self.status_message = "Import successful".to_string();
                    }
                    Err(e) => self.status_message = format!("Parse failed: {}", e),
                },
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddFolder => {
                return self.add_folder();
            }
            Message::FolderSelected(path) => {
                self.handle_folder_selected(path);
            }
            Message::RemoveSelected => {
                if let Some(index) = self.selected_index {
                    if index < self.batch_folders.len() {
                        self.batch_folders.remove(index);
                        self.selected_index = None;
                    }
                }
            }
            Message::ClearAll => {
                self.batch_folders.clear();
                self.selected_index = None;
            }
            Message::ExportBatch => {
                return self.export_batch();
            }
            Message::BatchExportSelected(path) => {
                self.handle_batch_export(path);
            }
            Message::ImportBatch => {
                return self.import_batch();
            }
            Message::BatchImportSelected(path) => {
                self.handle_batch_import(path);
            }
            Message::Execute => {
                self.execute();
            }
            Message::SelectFolder(index) => {
                self.selected_index = Some(index);
            }
            Message::SelectProfile(profile) => {
                if let Some(index) = self.selected_index {
                    if let Some(folder) = self.batch_folders.get_mut(index) {
                        folder.profile = profile;
                    }
                }
            }
            Message::SelectFolderProfile(folder_index, profile) => {
                if let Some(folder) = self.batch_folders.get_mut(folder_index) {
                    folder.profile = profile;
                }
            }
            Message::ToggleRecursive(value) => {
                self.gui_settings.do_recursive = value;
            }
            Message::ToggleAlign(value) => {
                self.gui_settings.do_align = value;
            }
            Message::ToggleFolderAlign(index, value) => {
                if index < self.batch_folders.len() {
                    self.batch_folders[index].align = value;
                }
            }
            Message::ToggleCleanup(value) => {
                self.gui_settings.do_cleanup = value;
            }
            Message::ThreadsChanged(value) => {
                self.gui_settings.threads = value.parse().unwrap_or(1);
            }

            // Setup dialog
            Message::OpenSetup => {
                self.setup_dialog.open(&self.config);
                self.profile_manager_dialog.close();
            }
            Message::SetupDialogMsg(msg) => {
                if let DialogMessage::Save = msg {
                    self.setup_dialog.save(&mut self.config);
                    if let Err(e) = self.config.save("config.json") {
                        self.status_message = format!("Failed to save config: {}", e);
                    } else {
                        self.status_message = "Configuration saved".to_string();

                        // Update main app state with new config
                        self.gui_settings.threads = self.config.gui_settings.threads as u32;
                        self.gui_settings.do_recursive = self.config.gui_settings.do_recursive;
                        self.gui_settings.do_cleanup = self.config.gui_settings.do_cleanup;
                        self.gui_settings.do_align = self.config.gui_settings.do_align;
                        self.gui_settings.use_opencv_align = self.config.gui_settings.use_opencv_align;
                        self.gui_settings.use_blender_merge = self.config.gui_settings.use_blender_merge;
                        self.gui_settings.use_opencv_debevec = self.config.gui_settings.use_opencv_debevec;

                        // Update profiles from config
                        self.profiles = if self.config.pp3_profiles.is_empty() {
                            vec![
                                "Default".to_string(),
                                "Landscape".to_string(),
                                "Portrait".to_string(),
                                "Night".to_string(),
                            ]
                        } else {
                            self.config.pp3_profiles.iter().map(|p| p.name.clone()).collect()
                        };
                    }
                    self.setup_dialog.cancel();
                } else if let DialogMessage::Cancel = msg {
                    self.setup_dialog.cancel();
                } else if let DialogMessage::BrowseAlignImageStackPath = msg {
                    let current_path = self.setup_dialog.get_align_image_stack_exe().to_string();
                    return Task::perform(
                        async move {
                            let mut file_dialog = rfd::AsyncFileDialog::new()
                                .add_filter("Executable", &["exe", ""]);
                            if !current_path.is_empty() {
                                if let Some(parent) = std::path::Path::new(&current_path).parent() {
                                    file_dialog = file_dialog.set_directory(parent);
                                }
                            }
                            file_dialog.pick_file().await
                        },
                        |file| {
                            Message::SetupDialogMsg(DialogMessage::AlignImageStackPathChanged(
                                file.map(|f| f.path().to_string_lossy().to_string()).unwrap_or_default()
                            ))
                        },
                    );
                } else if let DialogMessage::BrowseBlenderPath = msg {
                    let current_path = self.setup_dialog.get_blender_exe().to_string();
                    return Task::perform(
                        async move {
                            let mut file_dialog = rfd::AsyncFileDialog::new()
                                .add_filter("Executable", &["exe", ""]);
                            if !current_path.is_empty() {
                                if let Some(parent) = std::path::Path::new(&current_path).parent() {
                                    file_dialog = file_dialog.set_directory(parent);
                                }
                            }
                            file_dialog.pick_file().await
                        },
                        |file| {
                            Message::SetupDialogMsg(DialogMessage::BlenderPathChanged(
                                file.map(|f| f.path().to_string_lossy().to_string()).unwrap_or_default()
                            ))
                        },
                    );
                } else if let DialogMessage::BrowseLuminancePath = msg {
                    let current_path = self.setup_dialog.get_luminance_cli_exe().to_string();
                    return Task::perform(
                        async move {
                            let mut file_dialog = rfd::AsyncFileDialog::new()
                                .add_filter("Executable", &["exe", ""]);
                            if !current_path.is_empty() {
                                if let Some(parent) = std::path::Path::new(&current_path).parent() {
                                    file_dialog = file_dialog.set_directory(parent);
                                }
                            }
                            file_dialog.pick_file().await
                        },
                        |file| {
                            Message::SetupDialogMsg(DialogMessage::LuminancePathChanged(
                                file.map(|f| f.path().to_string_lossy().to_string()).unwrap_or_default()
                            ))
                        },
                    );
                } else if let DialogMessage::BrowseRawtherapeePath = msg {
                    let current_path = self.setup_dialog.get_rawtherapee_cli_exe().to_string();
                    return Task::perform(
                        async move {
                            let mut file_dialog = rfd::AsyncFileDialog::new()
                                .add_filter("Executable", &["exe", ""]);
                            if !current_path.is_empty() {
                                if let Some(parent) = std::path::Path::new(&current_path).parent() {
                                    file_dialog = file_dialog.set_directory(parent);
                                }
                            }
                            file_dialog.pick_file().await
                        },
                        |file| {
                            Message::SetupDialogMsg(DialogMessage::RawtherapeePathChanged(
                                file.map(|f| f.path().to_string_lossy().to_string()).unwrap_or_default()
                            ))
                        },
                    );
                } else if let DialogMessage::ThemeChanged(theme_name) = msg {
                    // Forward theme change to main app
                    if let Some(theme) = Theme::ALL.iter().find(|t| format!("{:?}", t) == theme_name) {
                        self.theme = Some(theme.clone());
                        self.config.gui_settings.theme_name = theme_name;
                        let _ = self.config.save("config.json");
                    }
                } else if let DialogMessage::PreviousTheme = msg {
                    // Handle PreviousTheme from setup dialog
                    let current = Theme::ALL.iter().position(
                        |candidate| self.theme.as_ref() == Some(candidate)
                    );
                    let new_idx = current.map(|c| if c == 0 { Theme::ALL.len() - 1 } else { c - 1 }).unwrap_or(0);
                    self.theme = Some(Theme::ALL[new_idx].clone());
                    self.config.gui_settings.theme_name = format!("{:?}", self.theme.as_ref().unwrap());
                    let _ = self.config.save("config.json");
                } else if let DialogMessage::NextTheme = msg {
                    // Handle NextTheme from setup dialog
                    let current = Theme::ALL.iter().position(
                        |candidate| self.theme.as_ref() == Some(candidate)
                    );
                    let new_idx = current.map(|c| (c + 1) % Theme::ALL.len()).unwrap_or(0);
                    self.theme = Some(Theme::ALL[new_idx].clone());
                    self.config.gui_settings.theme_name = format!("{:?}", self.theme.as_ref().unwrap());
                    let _ = self.config.save("config.json");
                } else if let DialogMessage::ClearTheme = msg {
                    // Handle ClearTheme from setup dialog
                    self.theme = None;
                    self.config.gui_settings.theme_name = String::new();
                    let _ = self.config.save("config.json");
                } else if let DialogMessage::UIScaleIncreased = msg {
                    // Handle uiscale increase from setup dialog
                    self.uiscale = (self.uiscale + 0.1).min(3.0);
                    self.config.gui_settings.uiscale = self.uiscale;
                    let _ = self.config.save("config.json");
                } else if let DialogMessage::UIScaleDecreased = msg {
                    // Handle uiscale decrease from setup dialog
                    self.uiscale = (self.uiscale - 0.1).max(0.5);
                    self.config.gui_settings.uiscale = self.uiscale;
                    let _ = self.config.save("config.json");
                } else if let DialogMessage::UIScaleSliderChanged(_) = msg {
                    // Slider is being dragged - value is tracked locally in SetupDialog
                    // Don't update parent uiscale until release
                } else if let DialogMessage::UIScaleSliderReleased(value) = msg {
                    // Handle uiscale slider release from setup dialog
                    self.uiscale = value.clamp(0.5, 3.0);
                    self.config.gui_settings.uiscale = self.uiscale;
                    let _ = self.config.save("config.json");
                } else {
                    self.setup_dialog.update(msg, &mut self.config);
                }
            }

            // Profile manager
            Message::OpenProfileManager => {
                self.profile_manager_dialog.open();
                self.setup_dialog.cancel();
            }
            Message::ProfileManagerMsg(msg) => {
                match msg {
                    ProfileMessage::Add => {
                        return Task::perform(
                            async {
                                let file_dialog = rfd::AsyncFileDialog::new()
                                    .add_filter("PP3 File", &["pp3"]);
                                file_dialog.pick_file().await
                            },
                            |file| {
                                file.map(|f| f.path().to_string_lossy().to_string())
                            },
                        ).map(Message::ProfileFileSelected);
                    }
                    ProfileMessage::Delete(index) => {
                        if index < self.config.pp3_profiles.len() {
                            self.config.pp3_profiles.remove(index);
                        }
                        if index < self.profiles.len() {
                            self.profiles.remove(index);
                        }
                        let _ = self.config.save("config.json");
                    }
                    ProfileMessage::Edit(index) => {
                        let profile = self.config.pp3_profiles.get(index);
                        let (name, path, tag) = if let Some(p) = profile {
                            (p.name.clone(), p.file_path.clone(), p.tag.clone())
                        } else {
                            (
                                self.profiles.get(index).cloned().unwrap_or_default(),
                                String::new(),
                                String::new(),
                            )
                        };
                        self.edit_profile_dialog.open(index, &name, &path, &tag);
                    }
                    ProfileMessage::ClearAll => {
                        self.clear_profiles_confirm_dialog.open();
                    }
                    ProfileMessage::Close => {
                        self.profile_manager_dialog.close();
                    }
                    ProfileMessage::NameEdit(value, index) => {
                        if index < self.config.pp3_profiles.len() {
                            self.config.pp3_profiles[index].name = value.clone();
                        }
                        if index < self.profiles.len() {
                            self.profiles[index] = value;
                        }
                        let _ = self.config.save("config.json");
                    }
                    ProfileMessage::PathEdit(value, index) => {
                        if index < self.config.pp3_profiles.len() {
                            self.config.pp3_profiles[index].file_path = value;
                        }
                        let _ = self.config.save("config.json");
                    }
                    ProfileMessage::TagEdit(value, index) => {
                        if index < self.config.pp3_profiles.len() {
                            self.config.pp3_profiles[index].tag = value;
                        }
                        let _ = self.config.save("config.json");
                    }
                }
            }
            Message::ProfileFileSelected(path_opt) => {
                if let Some(path_str) = path_opt {
                    let name = Path::new(&path_str)
                        .file_stem()
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

            // Edit profile
            Message::EditProfileMsg(msg) => {
                if let EditProfileMessage::Save = msg {
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
                } else if let EditProfileMessage::Cancel = msg {
                    self.edit_profile_dialog.close();
                } else {
                    self.edit_profile_dialog.update(msg);
                }
            }

            // Clear profiles
            Message::ClearProfilesMsg(msg) => {
                match msg {
                    ClearProfilesMessage::Confirm => {
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
                    ClearProfilesMessage::Cancel => {
                        self.clear_profiles_confirm_dialog.close();
                    }
                }
            }

            // UI scaling
            Message::FontSizeIncreased => {
                self.uiscale = (self.uiscale + 0.1).min(3.0); // Cap at 3x
                self.config.gui_settings.uiscale = self.uiscale;
                let _ = self.config.save("config.json");
            }
            Message::FontSizeDecreased => {
                self.uiscale = (self.uiscale - 0.1).max(0.1); // Minimum 0.5x
                self.config.gui_settings.uiscale = self.uiscale;
                let _ = self.config.save("config.json");
            }

            // Theme
            Message::ThemeChanged(theme) => {
                self.theme = Some(theme.clone());
                self.config.gui_settings.theme_name = format!("{:?}", theme);
                let _ = self.config.save("config.json");
            }
            Message::PreviousTheme | Message::NextTheme => {
                let current = Theme::ALL.iter().position(
                    |candidate| self.theme.as_ref() == Some(candidate)
                );

                self.theme = Some(
                    if matches!(message, Message::NextTheme) {
                        Theme::ALL[
                            current.map(|current| current + 1).unwrap_or(0) % Theme::ALL.len()
                        ].clone()
                    } else {
                        let current = current.unwrap_or(0);

                        if current == 0 {
                            Theme::ALL.last().expect("Theme::ALL must be empty").clone()
                        } else {
                            Theme::ALL[current - 1].clone()
                        }
                    }
                );
                if let Some(ref theme) = self.theme {
                    self.config.gui_settings.theme_name = format!("{:?}", theme);
                    let _ = self.config.save("config.json");
                }
            }
            Message::ClearTheme => {
                self.theme = None;
                self.config.gui_settings.theme_name = String::new();
                let _ = self.config.save("config.json");
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = Column::new().spacing(8.0 * self.uiscale).padding(10.0 * self.uiscale);

        // ========== Batch Folders Section ==========
        let folders_label = text("Input Folders:").size(14.0 * self.uiscale);

        let mut folder_rows = Column::new().spacing(4.0 * self.uiscale);

        // Header row
        let header = Row::new()
            .push(text("Folder").size(12.0 * self.uiscale).width(Length::Fill))
            .push(text("Profile").size(12.0 * self.uiscale).width(150.0 * self.uiscale))
            .push(text("Ext").size(12.0 * self.uiscale).width(50.0 * self.uiscale))
            .push(text("RAW").size(12.0 * self.uiscale).width(50.0 * self.uiscale))
            .push(text("Align").size(12.0 * self.uiscale).width(50.0 * self.uiscale))
            .push(text("Brackets").size(12.0 * self.uiscale).width(50.0 * self.uiscale))
            .push(text("Sets").size(12.0 * self.uiscale).width(50.0 * self.uiscale))
            .spacing(10.0 * self.uiscale)
            .padding([4.0 * self.uiscale, 10.0 * self.uiscale]);
        folder_rows = folder_rows.push(header);

        // Folder rows
        for (i, folder) in self.batch_folders.iter().enumerate() {
            let is_selected = self.selected_index == Some(i);
            let folder_row = Row::new()
                .push(
                    button(text(if is_selected {
                        format!("{}", &folder.path)
                    } else {
                        format!("{}", &folder.path)
                    }).size(12.0 * self.uiscale))
                        .style(if is_selected{ button::primary } else { button::secondary })
                        .on_press(Message::SelectFolder(i))
                        .width(Length::Fill)
                )
                .push(
                    pick_list(
                        &self.profiles[..],
                        Some(&folder.profile),
                        move |profile| Message::SelectFolderProfile(i, profile),
                    ).width(150.0 * self.uiscale)
                )
                .push(text(&folder.extension).size(12.0 * self.uiscale).width(50.0 * self.uiscale))
                .push(text(if folder.is_raw { "Yes" } else { "No" }).size(12.0 * self.uiscale).width(50.0 * self.uiscale))
                .push(container(checkbox(folder.align).on_toggle(move |value| Message::ToggleFolderAlign(i, value))).width(50.0 * self.uiscale))
                .push(text(folder.brackets.to_string()).width(50.0 * self.uiscale).size(12.0 * self.uiscale))
                .push(text(format!("{} files", folder.files.len())).size(12.0 * self.uiscale).width(50.0 * self.uiscale))
                .spacing(10.0 * self.uiscale)
                .padding([4.0 * self.uiscale, 10.0 * self.uiscale]);
            folder_rows= folder_rows.push(horizontal_rule(2));
            folder_rows = folder_rows.push(folder_row);
        }

        let folder_scroll = container((folder_rows).height(Length::Fixed(250.0 * self.uiscale))).style(container::bordered_box)
            ;

        let buttons = Column::new()
            .spacing(10.0 * self.uiscale)
            .width(120.0 * self.uiscale)
            .padding(10.0 * self.uiscale)
            .push(button(text("Add").size(16.0 * self.uiscale)).on_press(Message::AddFolder).style(button::success).width(100.0 * self.uiscale))
            .push(button(text("Remove").size(16.0 * self.uiscale)).on_press(Message::RemoveSelected).style(button::warning).width(100.0 * self.uiscale))
            .push(button(text("Clear All").size(16.0 * self.uiscale)).on_press(Message::ClearAll).style(button::danger).width(100.0 * self.uiscale))
            .push(checkbox(self.gui_settings.do_recursive).label("Recursive").on_toggle(Message::ToggleRecursive), )
            .push(horizontal_rule((2.0 * self.uiscale) as u16))
            .push(button(text("Export").size(16.0 * self.uiscale)).on_press(Message::ExportBatch).style(button::secondary).width(100.0 * self.uiscale))
            .push(button(text("Import").size(16.0 * self.uiscale)).on_press(Message::ImportBatch).style(button::secondary).width(100.0 * self.uiscale))
            ;


        let folders_section =
            row!(
                folder_scroll,
                buttons
            ).spacing(10.0 * self.uiscale);


        content = content.push(Column::new().push(folders_label).push(folders_section).spacing(8.0 * self.uiscale));

        // Show files for selected folder
        if let Some(index) = self.selected_index {
            if let Some(folder) = self.batch_folders.get(index) {
                content = content.push(horizontal_rule((2.0 * self.uiscale) as u16));
                let files_label = text(format!(
                    "Files in: {} ({} files, {} brackets, {} sets)",
                    folder.path,
                    folder.files.len(),
                    folder.brackets,
                    folder.sets
                )).size(14.0 * self.uiscale);

                let mut file_rows = Column::new().spacing(4.0 * self.uiscale);
                let file_header = Row::new()
                    .push(text("File Path").width(Length::Fill))
                    .push(text("EXIF Info").width(Length::Fill))
                    .spacing(10.0 * self.uiscale)
                    .padding([4.0 * self.uiscale, 10.0 * self.uiscale]);
                file_rows = file_rows.push(file_header);

                for file in &folder.files {
                    let exif_info = Self::format_exif_info(file);
                    let file_row = Row::new()
                        .push(text(&file.path).width(Length::Fill))
                        .push(text(if exif_info.is_empty() {
                            "No EXIF".to_string()
                        } else {
                            exif_info
                        }).width(Length::Fill))
                        .spacing(10.0 * self.uiscale)
                        .padding([4.0 * self.uiscale, 10.0 * self.uiscale]);
                    file_rows = file_rows.push(file_row);
                }

                let file_scroll = scrollable(file_rows).height(Length::Fixed(150.0 * self.uiscale));
                content = content.push(Column::new().push(files_label).push(file_scroll).spacing(4.0 * self.uiscale));
            }
        }

        content = content.push(horizontal_rule((2.0 * self.uiscale) as u16));

        // ========== Profile Selection Section ==========
        let profile_label = text("PP3 Profile:").size(14.0 * self.uiscale);

        let selected_profile = self
            .selected_index
            .and_then(|i| self.batch_folders.get(i))
            .map(|f| f.profile.clone())
            .unwrap_or_else(|| "Select...".to_string());

        let profile_picklist = pick_list(
            &self.profiles[..],
            Some(selected_profile),
            Message::SelectProfile,
        ).placeholder("Select...");

        let align_text = if self.gui_settings.use_opencv_align {
            "Align (OpenCV)"
        } else {
            "Align (Hugin)"
        };
        let align_checkbox = checkbox(self.gui_settings.do_align)
            .label(align_text)
            .on_toggle(Message::ToggleAlign);

        let manage_profiles_btn = button(text("Manage Profiles...").size(16.0 * self.uiscale))
            .on_press(Message::OpenProfileManager)
            .style(button::secondary);

        let profile_section = Row::new()
            .push(profile_label)
            .push(profile_picklist)
            .push(align_checkbox)
            .push(horizontal_space())
            .push(manage_profiles_btn)
            .spacing(10.0 * self.uiscale)
            .align_y(Alignment::Center);
        content = content.push(profile_section);

        content = content.push(horizontal_rule((2.0 * self.uiscale) as u16));

        // ========== Options Section ==========
        // let threads_label = text("Threads:").size(14.0 * self.uiscale);
        // let threads_input = text_input("Threads", &self.gui_settings.threads.to_string())
        //     .on_input(Message::ThreadsChanged)
        //     .width(Length::Fixed(60.0 * self.uiscale));

        // let cleanup_checkbox = checkbox(self.gui_settings.do_cleanup)
        //     .label("Cleanup temporary files")
        //     .on_toggle(Message::ToggleCleanup);

        let create_hdrs_btn = button(text("Create HDRs").size(16.0 * self.uiscale)).on_press(Message::Execute);
        let setup_btn = button(text("Setup").size(16.0 * self.uiscale)).on_press(Message::OpenSetup).style(button::secondary);

        let options_section = Row::new()
            // .push(threads_label)
            // .push(threads_input)
            // .push(cleanup_checkbox)
            .push(horizontal_space())
            .push(create_hdrs_btn)
            .push(setup_btn)
            .spacing(10.0 * self.uiscale)
            .align_y(Alignment::Center);
        content = content.push(options_section);


        // ========== Progress Bar Section ==========
        let progress_bar_widget = container(progress_bar(0.0..=1.0, self.progress)).height(2.0 * self.uiscale);
        let status_text = text(&self.status_message).size(16.0 * self.uiscale);
        let progress_section = Row::new()
            .push(progress_bar_widget)
            .push(status_text)
            .spacing(10.0 * self.uiscale)
            .align_y(Alignment::Center);
        content = content.push(progress_section);

        // Build the main content
        let main_content = container(content).width(Length::Fill).height(Length::Fill);

        // Overlay dialogs if they are open
        if self.setup_dialog.show {
            return container(
                Column::new()
                    .push(main_content)
                    .push(
                        container(self.setup_dialog.view(&self.config, self.uiscale).map(Message::SetupDialogMsg))
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        if self.profile_manager_dialog.show {
            return container(
                Column::new()
                    .push(main_content)
                    .push(
                        container(self.profile_manager_dialog.view(&self.profiles, &self.config.pp3_profiles, self.uiscale).map(Message::ProfileManagerMsg))
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        if self.edit_profile_dialog.show {
            return container(
                Column::new()
                    .push(main_content)
                    .push(
                        container(self.edit_profile_dialog.view(self.uiscale).map(Message::EditProfileMsg))
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        if self.clear_profiles_confirm_dialog.show {
            return container(
                Column::new()
                    .push(main_content)
                    .push(
                        container(self.clear_profiles_confirm_dialog.view(self.uiscale).map(Message::ClearProfilesMsg))
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        main_content.into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, repeat: false, .. } = event else {
                return None;
            };

            match key {
                keyboard::Key::Character(c) if c.as_str() == "=" || c.as_str() == "+" => {
                    Some(Message::FontSizeIncreased)
                }
                keyboard::Key::Character(c) if c.as_str() == "-" => {
                    Some(Message::FontSizeDecreased)
                }
                _ => None,
            }
        })
    }

    pub fn theme(&self) -> Option<Theme> {
        self.theme.clone()
    }
}

fn horizontal_space() -> Element<'static, Message> {
    space().width(Fill).into()
    // Space::new().width(Fill()).into()
}

fn horizontal_rule(thickness: u16) -> Element<'static, Message> {
    rule::horizontal(thickness as u32).into()
}
