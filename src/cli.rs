//! Command-line interface for headless batch processing

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// HDR Merge Master - Batch HDR processing from bracketed images
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Run in headless CLI mode (no GUI)
    #[arg(long)]
    pub cli: bool,

    /// Load batch folder list from a JSON file
    #[arg(short = 'b', long, value_name = "FILE")]
    pub batch: Option<PathBuf>,

    /// Add a single folder to process
    #[arg(short = 'f', long, value_name = "PATH")]
    pub folder: Option<PathBuf>,

    /// Process subfolders recursively (with --folder)
    #[arg(short = 'r', long, requires = "folder")]
    pub recursive: bool,

    /// PP3 profile name to use (for RAW files)
    #[arg(short = 'p', long, value_name = "NAME")]
    pub profile: Option<String>,

    /// Enable image alignment
    #[arg(short = 'a', long)]
    pub align: bool,

    /// Number of worker threads (default: 6)
    #[arg(short = 't', long, value_name = "N", default_value = "6")]
    pub threads: u32,

    /// Cleanup temporary files after processing
    #[arg(short = 'c', long)]
    pub cleanup: bool,

    /// Print detailed progress information
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Use OpenCV AlignMTB instead of align_image_stack
    #[arg(long)]
    pub use_opencv_align: bool,

    /// Use OpenCV MergeDebevec instead of Blender
    #[arg(long)]
    pub use_opencv_debevec: bool,

    /// Use OpenCV MergeRobertson instead of Blender
    #[arg(long)]
    pub use_opencv_merge_robertson: bool,

    /// Use OpenCV tone mapping instead of Luminance CLI
    #[arg(long)]
    pub use_opencv_tonemap: bool,

    /// Tone mapping operator (Reinhard, Drago, Durand, Mantiuk)
    #[arg(long, default_value = "Reinhard")]
    pub tonemap_operator: String,

    /// Tone mapping intensity (0.0-1.0)
    #[arg(long, default_value = "1.0")]
    pub tonemap_intensity: f32,

    /// Tone mapping contrast (0.0-1.0)
    #[arg(long, default_value = "1.0")]
    pub tonemap_contrast: f32,

    /// Tone mapping saturation (0.0-1.0)
    #[arg(long, default_value = "1.0")]
    pub tonemap_saturation: f32,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Process folders directly
    Process {
        /// Folder paths to process
        #[arg(required = true)]
        folders: Vec<PathBuf>,
    },
}

/// Batch file format for --batch option
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct BatchEntry {
    /// Folder path to process
    pub path: String,
    /// Profile name to use (optional, uses default if not specified)
    #[serde(default)]
    pub profile: Option<String>,
    /// Enable alignment (optional, uses config default if not specified)
    #[serde(default)]
    pub align: Option<bool>,
    /// Extension filter (optional)
    #[serde(default)]
    pub extension: Option<String>,
}

/// Batch file root structure
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct BatchFile {
    /// List of batch entries
    pub folders: Vec<BatchEntry>,
}

impl Cli {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// Check if CLI mode is enabled
    pub fn is_cli_mode(&self) -> bool {
        self.cli
    }
}
