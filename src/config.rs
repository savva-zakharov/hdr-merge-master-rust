//! Configuration structures for the application

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::scan_folder::ScannedFile;

/// Get the configuration file path
pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = if cfg!(target_os = "windows") {
        // Windows: %APPDATA%\hdr-merge-master\
        dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("hdr-merge-master")
    } else {
        // Linux/Mac: ~/.config/hdr-merge-master/
        dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("hdr-merge-master")
    };

    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.json"))
}

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

impl Default for Config {
    fn default() -> Self {
        Config {
            _needs_setup: true,
            _optional_exes_available: OptionalExesAvailable::default(),
            exe_paths: ExePaths::default(),
            gui_settings: GuiSettingsConfig::default(),
            pp3_profiles: Vec::new(),
        }
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
    pub use_opencv_align: bool,
    #[serde(default)]
    pub use_opencv_merge: bool,  // Use OpenCV MergeDebevec instead of Blender
    #[serde(default)]
    pub use_opencv_merge_robertson: bool,  // Use OpenCV MergeRobertson instead of Blender (alternative to MergeDebevec)
    #[serde(default)]
    pub use_opencv_tonemap: bool,  // Use OpenCV tone mapping instead of Luminance CLI
    #[serde(default = "default_tonemap_operator")]
    pub tonemap_operator: String,  // Reinhard, Drago, Durand, Mantiuk
    #[serde(default = "default_tonemap_intensity")]
    pub tonemap_intensity: f32,
    #[serde(default = "default_tonemap_contrast")]
    pub tonemap_contrast: f32,
    #[serde(default = "default_tonemap_saturation")]
    pub tonemap_saturation: f32,
    #[serde(default = "default_uiscale")]
    pub uiscale: f32,
    #[serde(default = "default_theme_name")]
    pub theme_name: String,
}

fn default_tonemap_operator() -> String {
    "Reinhard".to_string()
}

fn default_tonemap_intensity() -> f32 {
    1.0
}

fn default_tonemap_contrast() -> f32 {
    1.0
}

fn default_tonemap_saturation() -> f32 {
    1.0
}

fn default_recursive_max_depth() -> u32 {
    1
}

fn default_threads() -> u8 {
    6
}

fn default_uiscale() -> f32 {
    1.0
}

fn default_theme_name() -> String {
    String::new() // Empty means system default
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
            use_opencv_align: false,
            use_opencv_merge: false,
            use_opencv_merge_robertson: false,
            use_opencv_tonemap: false,
            tonemap_operator: "Reinhard".to_string(),
            tonemap_intensity: 1.0,
            tonemap_contrast: 1.0,
            tonemap_saturation: 1.0,
            uiscale: 1.0,
            theme_name: String::new(),
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
    pub use_opencv_align: bool,
    pub use_opencv_merge: bool,
    pub use_opencv_merge_robertson: bool,
    pub use_opencv_tonemap: bool,
    pub tonemap_operator: String,
    pub tonemap_intensity: f32,
    pub tonemap_contrast: f32,
    pub tonemap_saturation: f32,
}

impl Default for GuiSettings {
    fn default() -> Self {
        GuiSettings {
            threads: 6,
            do_recursive: false,
            do_cleanup: false,
            do_align: false,
            use_opencv_align: false,
            use_opencv_merge: false,
            use_opencv_merge_robertson: false,
            use_opencv_tonemap: false,
            tonemap_operator: "Reinhard".to_string(),
            tonemap_intensity: 1.0,
            tonemap_contrast: 1.0,
            tonemap_saturation: 1.0,
        }
    }
}

impl From<&GuiSettingsConfig> for GuiSettings {
    fn from(config: &GuiSettingsConfig) -> Self {
        GuiSettings {
            threads: config.threads as u32,
            do_recursive: config.do_recursive,
            do_cleanup: config.do_cleanup,
            do_align: config.do_align,
            use_opencv_align: config.use_opencv_align,
            use_opencv_merge: config.use_opencv_merge,
            use_opencv_merge_robertson: config.use_opencv_merge_robertson,
            use_opencv_tonemap: config.use_opencv_tonemap,
            tonemap_operator: config.tonemap_operator.clone(),
            tonemap_intensity: config.tonemap_intensity,
            tonemap_contrast: config.tonemap_contrast,
            tonemap_saturation: config.tonemap_saturation,
        }
    }
}
