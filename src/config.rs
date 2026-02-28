//! Configuration structures for the application

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::scan_folder::ScannedFile;

// ========== Configuration File Structures ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub _needs_setup: bool,
    #[serde(default)]
    pub _optional_exes_available: OptionalExesAvailable,
    pub exe_paths: ExePaths,
    pub gui_settings: GuiSettingsConfig,
    #[serde(default)]
    pub pp3_profiles: Vec<Profile>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalExesAvailable {
    #[serde(default)]
    pub align_image_stack_exe: bool,
    #[serde(default)]
    pub rawtherapee_cli_exe: bool,
}

impl Default for OptionalExesAvailable {
    fn default() -> Self {
        Self {
            align_image_stack_exe: false,
            rawtherapee_cli_exe: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExePaths {
    #[serde(default)]
    pub align_image_stack_exe: String,
    #[serde(default)]
    pub blender_exe: String,
    #[serde(default)]
    pub luminance_cli_exe: String,
    #[serde(default)]
    pub rawtherapee_cli_exe: String,
}

impl Default for ExePaths {
    fn default() -> Self {
        Self {
            align_image_stack_exe: String::new(),
            blender_exe: String::new(),
            luminance_cli_exe: String::new(),
            rawtherapee_cli_exe: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettingsConfig {
    #[serde(default)]
    pub do_align: bool,
    #[serde(default)]
    pub do_cleanup: bool,
    #[serde(default)]
    pub do_raw: bool,
    #[serde(default)]
    pub do_recursive: bool,
    #[serde(default)]
    pub pp3_file: String,
    #[serde(default)]
    pub processed_extensions: Vec<String>,
    #[serde(default)]
    pub raw_extensions: Vec<String>,
    #[serde(default)]
    pub recursive_ignore_folders: Vec<String>,
    #[serde(default = "default_recursive_max_depth")]
    pub recursive_max_depth: u32,
    #[serde(default = "default_threads")]
    pub threads: u8,
    #[serde(default)]
    pub use_opencv: bool,
}

fn default_recursive_max_depth() -> u32 {
    1
}

fn default_threads() -> u8 {
    6
}

impl Default for GuiSettingsConfig {
    fn default() -> Self {
        Self {
            do_align: false,
            do_cleanup: false,
            do_raw: false,
            do_recursive: false,
            pp3_file: String::new(),
            processed_extensions: vec![
                ".tif".to_string(),
                ".tiff".to_string(),
                ".png".to_string(),
            ],
            raw_extensions: vec![
                ".dng".to_string(),
                ".cr2".to_string(),
                ".cr3".to_string(),
                ".nef".to_string(),
                ".arw".to_string(),
                ".raf".to_string(),
                ".orf".to_string(),
                ".rw2".to_string(),
                ".pef".to_string(),
            ],
            recursive_ignore_folders: vec![
                "Merged".to_string(),
                "tif".to_string(),
                "exr".to_string(),
                "jpg".to_string(),
                "aligned".to_string(),
            ],
            recursive_max_depth: 1,
            threads: 6,
            use_opencv: false,
        }
    }
}

// ========== Application Structures ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub exif_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub file_path: String,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub path: String,
    pub profile: String,
    pub extension: String,
    pub is_raw: bool,
    pub align: bool,
    pub brackets: u32,
    pub sets: u32,
    pub files: Vec<ScannedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    pub threads: u32,
    pub do_recursive: bool,
    pub do_cleanup: bool,
    pub do_align: bool,
    pub use_opencv: bool,
}

impl Default for GuiSettings {
    fn default() -> Self {
        GuiSettings {
            threads: 6,
            do_recursive: false,
            do_cleanup: false,
            do_align: false,
            use_opencv: false,
        }
    }
}
