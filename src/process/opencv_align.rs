//! OpenCV-based image alignment using libstacker
//!
//! This module provides an alternative to align_image_stack using libstacker's
//! ECC (Enhanced Correlation Coefficient) or KeyPoint matching algorithms.
//!
//! **NOTE**: This module requires OpenCV to be installed on your system.
//! See OPENCV_SETUP.md for installation instructions.

use std::path::{Path, PathBuf};
use std::time::Instant;

/// Align images using libstacker's ECC matching algorithm
///
/// ECC (Enhanced Correlation Coefficient) is robust for exposure differences,
/// making it ideal for HDR bracket alignment.
///
/// # Arguments
/// * `source_files` - List of source file paths to align (one bracket set)
/// * `align_folder` - Output directory for aligned files
/// * `set_idx` - Set index for naming output files
/// * `logs_dir` - Directory to save log files
///
/// # Returns
/// List of aligned file paths
pub fn align_set_with_opencv(
    source_files: &[PathBuf],
    align_folder: &Path,
    set_idx: usize,
    logs_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let set_start = Instant::now();
    
    println!("    [OPENCV] Aligning {} files with libstacker ECC...", source_files.len());

    // Create align output directory
    if let Err(e) = std::fs::create_dir_all(align_folder) {
        return Err(format!("Failed to create align directory: {}", e));
    }

    // TODO: Implement actual OpenCV alignment when libstacker is enabled
    // See OPENCV_SETUP.md for installation instructions
    
    // Placeholder implementation - copies files
    // When libstacker is enabled, this will use actual ECC alignment
    let mut aligned_files = Vec::new();
    for (idx, src_file) in source_files.iter().enumerate() {
        let src_data = std::fs::read(src_file)
            .map_err(|e| format!("Failed to read {}: {}", src_file.display(), e))?;
        
        let out_filename = format!("opencv_set_{}_{:04}.tif", set_idx, idx + 1);
        let out_path = align_folder.join(&out_filename);
        
        std::fs::write(&out_path, &src_data)
            .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;
        
        aligned_files.push(out_path);
    }

    // Create log entry
    let log_file = logs_dir.join(format!("opencv_align_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== OpenCV Alignment (PLACEHOLDER) - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Input files: {}\n", source_files.len()));
    for file in source_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str(&format!("\nOutput files: {}\n", aligned_files.len()));
    for file in &aligned_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str(&format!("\nProcessing time: {:.2}s\n", set_start.elapsed().as_secs_f32()));
    log_content.push_str("\n⚠️  NOTE: This is a PLACEHOLDER implementation.\n");
    log_content.push_str("To enable actual OpenCV/libstacker alignment:\n");
    log_content.push_str("1. Install LLVM: choco install llvm\n");
    log_content.push_str("2. Install CMake: choco install cmake\n");
    log_content.push_str("3. Install OpenCV via vcpkg (see OPENCV_SETUP.md)\n");
    log_content.push_str("4. Uncomment 'libstacker = \"0.1\"' in Cargo.toml\n");
    log_content.push_str("5. Rebuild: cargo build --release\n");

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    println!("    [OPENCV] Set {}: ✓ Complete (Time: {:.2}s) [PLACEHOLDER - copying files]", 
        set_idx, set_start.elapsed().as_secs_f32());

    Ok(aligned_files)
}

/// Alternative alignment using KeyPoint matching (placeholder)
///
/// This method uses feature detection (ORB/SIFT) and matching.
#[allow(dead_code)]
pub fn align_set_with_keypoints(
    _source_files: &[PathBuf],
    _align_folder: &Path,
    _set_idx: usize,
    _logs_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    Err("KeyPoint alignment not implemented. Enable libstacker in Cargo.toml.".to_string())
}
