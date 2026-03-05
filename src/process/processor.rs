//! HDR Processor implementation
//!
//! Handles the actual processing of bracketed images into HDR
//!
//! Processing Flow:
//! 1. If RAW files: process with RawTherapee CLI → Merged/tif/
//! 2. If align enabled: align with align_image_stack → Merged/aligned/ (or OpenCV AlignMTB if enabled)
//! 3. Merge each bracketed set using Blender HDR_Merge.blend → Merged/exr/ (or OpenCV MergeDebevec/Robertson if enabled)
//! 4. Tone map EXR to JPG using Luminance CLI → Merged/jpg/ (or OpenCV tonemapping if enabled)
//!
//! All steps log output to Merged/logs/

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::config::{Config, FolderEntry, GuiSettings};

/// Process all folders in the batch
///
/// # Arguments
/// * `folder` - Folder entry to process
/// * `config` - Application configuration
/// * `gui_settings` - GUI settings
///
/// # Returns
/// Result indicating success or error message
pub fn process_folder(
    folder: &FolderEntry,
    config: &Config,
    gui_settings: &GuiSettings,
) -> Result<String, String> {
    let folder_path = Path::new(&folder.path);

    if !folder_path.exists() {
        return Err(format!("Folder does not exist: {}", folder.path));
    }

    if folder.files.is_empty() {
        return Err(format!("No files to process in: {}", folder.path));
    }

    // Get the profile file path
    let profile = config
        .pp3_profiles
        .iter()
        .find(|p| p.name == folder.profile);

    let profile_path = profile.map(|p| p.file_path.clone());

    // Create Merged directory
    let merged_dir = folder_path.join("Merged");
    if let Err(e) = std::fs::create_dir_all(&merged_dir) {
        return Err(format!("Failed to create merged directory: {}", e));
    }

    // Create logs directory
    let logs_dir = merged_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
        return Err(format!("Failed to create logs directory: {}", e));
    }

    let total_start = Instant::now();
    println!("[PROCESS] Starting HDR processing for: {}", folder.path);
    println!(
        "[PROCESS] Folders: {} sets, {} brackets per set",
        folder.sets, folder.brackets
    );
    println!(
        "[PROCESS] Started at: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // Step 1: If RAW files, process with RawTherapee CLI to Merged/tif/
    let tif_folder = merged_dir.join("tif");
    let step_start = Instant::now();
    let source_files = if folder.is_raw {
        println!("[STEP 1] Processing RAW files with RawTherapee CLI...");
        process_raw_files(
            &folder.files,
            &tif_folder,
            &profile_path,
            &config.exe_paths.rawtherapee_cli_exe,
            &logs_dir,
        )?
    } else {
        println!("[STEP 1] Skipping RAW processing (non-RAW files)");
        // For non-RAW files, use them directly
        folder
            .files
            .iter()
            .map(|f| PathBuf::from(&f.path))
            .collect()
    };
    if folder.is_raw {
        println!(
            "[STEP 1] Completed in {:.2}s",
            step_start.elapsed().as_secs_f32()
        );
    }

    // Step 2: If alignment enabled, align each bracket set with align_image_stack to Merged/aligned/
    let aligned_files = if gui_settings.do_align {
        println!(
            "[STEP 2] Aligning {} bracket sets with align_image_stack ({} threads)...",
            folder.sets, gui_settings.threads
        );
        let step_start = Instant::now();
        let align_folder = merged_dir.join("aligned");
        let result = align_images_by_set_concurrent(
            &source_files,
            &align_folder,
            gui_settings.use_opencv_align,
            &config.exe_paths.align_image_stack_exe,
            folder,
            &logs_dir,
            gui_settings.threads as usize,
        )?;
        println!(
            "[STEP 2] Completed in {:.2}s",
            step_start.elapsed().as_secs_f32()
        );
        result
    } else {
        println!("[STEP 2] Skipping alignment (disabled)");
        source_files.clone()
    };

    // Step 3: Merge each bracketed set using Blender
    let exr_folder = merged_dir.join("exr");
    let source_files_for_merge = if gui_settings.do_align {
        // For aligned files, we need to reload from the aligned folder
        // since alignment outputs new files
        reload_aligned_files(&merged_dir.join("aligned"))?
    } else {
        // Use source files (either RAW originals or TIFFs from Step 1)
        // We need to convert back to ScannedFile format
        source_files
            .iter()
            .map(|p| crate::scan_folder::ScannedFile {
                path: p.to_string_lossy().to_string(),
                exposure_time: None,
                f_number: None,
                iso: None,
            })
            .collect()
    };

    // For EV calculation, we always use the original scanned files (folder.files)
    // This ensures we have accurate exposure information even after alignment
    // which creates new files without EXIF data
    let ev_source_files = folder.files.clone();

    // Step 3: Merge each bracketed set using either OpenCV MergeDebevec/Robertson or Blender
    if gui_settings.use_opencv_merge {
        println!(
            "[STEP 3] Merging {} bracket sets with OpenCV MergeDebevec ({} threads)...",
            folder.sets, gui_settings.threads
        );
    } else if gui_settings.use_opencv_merge_robertson {
        println!(
            "[STEP 3] Merging {} bracket sets with OpenCV MergeRobertson ({} threads)...",
            folder.sets, gui_settings.threads
        );
    } else {
        println!(
            "[STEP 3] Merging {} bracket sets with Blender ({} threads)...",
            folder.sets, gui_settings.threads
        );
    }
    let step_start = Instant::now();

    if gui_settings.use_opencv_merge {
        crate::process::opencv_merge::merge_with_opencv_debevec_concurrent(
            &aligned_files,
            &exr_folder,
            &ev_source_files,
            folder,
            &logs_dir,
            folder.sets,
            gui_settings.threads as usize,
        )?;
    } else if gui_settings.use_opencv_merge_robertson {
        crate::process::opencv_merge::merge_with_opencv_robertson_concurrent(
            &aligned_files,
            &exr_folder,
            &ev_source_files,
            folder,
            &logs_dir,
            folder.sets,
            gui_settings.threads as usize,
        )?;
    } else {
        crate::process::external_blender::merge_with_blender_concurrent(
            &aligned_files,
            &exr_folder,
            &source_files_for_merge,
            &ev_source_files,
            folder,
            &config.exe_paths.blender_exe,
            &logs_dir,
            folder.sets,
            gui_settings.threads as usize,
        )?;
    }
    println!(
        "[STEP 3] Completed in {:.2}s",
        step_start.elapsed().as_secs_f32()
    );

    // Wait a moment for all file handles to be released and files to be fully written
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Step 4: Tone map HDR to JPG using either OpenCV or Luminance CLI
    let jpg_folder = merged_dir.join("jpg");

    // Determine which files to tone map (EXR for Blender, TIFF for OpenCV merge)
    let hdr_folder = if gui_settings.use_opencv_merge {
        exr_folder.clone() // TIFF files from OpenCV merge
    } else {
        exr_folder.clone() // EXR files from Blender
    };

    if gui_settings.use_opencv_tonemap {
        println!(
            "[STEP 4] Tone mapping HDR to JPG with OpenCV ({} threads)...",
            gui_settings.threads
        );
        let step_start = Instant::now();
        tone_map_with_opencv(
            &hdr_folder,
            &jpg_folder,
            gui_settings,
            &logs_dir,
            gui_settings.threads as usize,
        )?;
        println!(
            "[STEP 4] Completed in {:.2}s",
            step_start.elapsed().as_secs_f32()
        );
    } else {
        println!(
            "[STEP 4] Tone mapping EXR files to JPG with Luminance CLI ({} threads)...",
            gui_settings.threads
        );
        let step_start = Instant::now();
        crate::process::external_luminance::tone_map_exr_to_jpg_concurrent(
            &exr_folder,
            &jpg_folder,
            &config.exe_paths.luminance_cli_exe,
            &logs_dir,
            gui_settings.threads as usize,
        )?;
        println!(
            "[STEP 4] Completed in {:.2}s",
            step_start.elapsed().as_secs_f32()
        );
    }

    // Step 5: Cleanup temporary files if enabled
    if gui_settings.do_cleanup {
        println!("[STEP 5] Cleaning up temporary files...");
        cleanup_temp_files(&merged_dir, gui_settings.do_align)?;
    } else {
        println!("[STEP 5] Skipping cleanup (disabled)");
    }

    println!("[PROCESS] ✓ Successfully processed: {}", folder.path);
    let total_elapsed = total_start.elapsed();
    println!(
        "[PROCESS] Total time: {:.2}s ({:.2} min)",
        total_elapsed.as_secs_f32(),
        total_elapsed.as_secs_f32() / 60.0
    );

    Ok(format!("Successfully processed: {}", folder.path))
}

/// Process RAW files with RawTherapee CLI
/// Outputs TIFF files to the tif_folder
///
/// Command: rawtherapee_cli -p pp3_file -o tif_folder -t -c raw_file1 raw_file2 ...
///
/// # Arguments
/// * `raw_files` - List of RAW files to process
/// * `tif_folder` - Output directory for TIFF files (Merged/tif/)
/// * `profile_path` - Optional PP3 profile path
/// * `rawtherapee_exe` - Path to RawTherapee CLI executable
/// * `logs_dir` - Directory to save log files
///
/// # Returns
/// List of generated TIFF file paths
fn process_raw_files(
    raw_files: &[crate::scan_folder::ScannedFile],
    tif_folder: &Path,
    profile_path: &Option<String>,
    rawtherapee_exe: &str,
    logs_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    // Create tif output directory
    if let Err(e) = std::fs::create_dir_all(tif_folder) {
        return Err(format!("Failed to create tif directory: {}", e));
    }

    // Check if we have a profile
    let Some(pp3_file) = profile_path else {
        return Err("No PP3 profile selected for RAW processing".to_string());
    };

    // Check if RawTherapee CLI is configured
    if rawtherapee_exe.is_empty() {
        return Err("RawTherapee CLI not configured in setup".to_string());
    }

    // Build RawTherapee CLI command
    // Command: rawtherapee_cli -p pp3_file -o tif_folder -t -c raw_file1 raw_file2 ...
    let mut cmd = Command::new(rawtherapee_exe);
    cmd.arg("-p")
        .arg(pp3_file)
        .arg("-o")
        .arg(tif_folder.to_str().ok_or("Invalid tif folder path")?)
        .arg("-t") // Use threads
        .arg("-c"); // Overwrite existing files

    // Add all RAW files to process
    for raw_file in raw_files {
        cmd.arg(&raw_file.path);
    }

    println!("  [RAW] Processing {} files...", raw_files.len());
    let step_start = Instant::now();

    // Execute command and capture output
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute RawTherapee CLI: {}", e))?;

    // Save logs
    let log_file = logs_dir.join("rawtherapee.log");
    let mut log_content = String::new();
    log_content.push_str("=== RawTherapee CLI Processing ===\n\n");
    log_content.push_str(&format!("Command: {:?}\n\n", cmd));
    log_content.push_str("STDOUT:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stdout));
    log_content.push_str("\nSTDERR:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stderr));

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("  [RAW] ✗ Failed!");
        return Err(format!("RawTherapee processing failed: {}", stderr));
    }

    println!(
        "  [RAW] ✓ Complete (Time: {:.2}s)",
        step_start.elapsed().as_secs_f32()
    );

    // Collect generated TIFF files
    let mut tif_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(tif_folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "tif"
                        || ext.to_string_lossy().to_lowercase() == "tiff"
                    {
                        tif_files.push(path);
                    }
                }
            }
        }
    }

    // Sort by filename for consistent ordering
    tif_files.sort();

    Ok(tif_files)
}

/// Align a single bracket set using align_image_stack
///
/// # Arguments
/// * `set_files` - Files in this bracket set to align
/// * `align_folder` - Output directory for aligned files
/// * `set_idx` - Set index for naming
/// * `align_exe` - Path to align_image_stack executable
/// * `logs_dir` - Directory to save log files
///
/// # Returns
/// List of aligned file paths for this set
fn align_single_set(
    set_files: &[PathBuf],
    align_folder: &Path,
    set_idx: usize,
    align_exe: &str,
    logs_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    // Build align_image_stack command for this set
    // Command: align_image_stack -v -i -l -a align_folder/align_set_N_ --gpu file1 file2 ...
    let mut cmd = Command::new(align_exe);
    cmd.arg("-v") // Verbose
        .arg("-i") // Auto-crop
        .arg("-l") // Keep linear values
        .arg("-a")
        .arg(
            align_folder
                .join(format!("align_set_{}_", set_idx))
                .to_str()
                .ok_or("Invalid align folder path")?,
        )
        .arg("--gpu");

    // Add files for this set only
    for file in set_files {
        cmd.arg(file);
    }

    // Execute command and capture output
    let output = cmd.output().map_err(|e| {
        format!(
            "Failed to execute align_image_stack for set {}: {}",
            set_idx, e
        )
    })?;

    // Save logs for this set
    let log_file = logs_dir.join(format!("align_image_stack_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== Align Image Stack - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Command: {:?}\n\n", cmd));
    log_content.push_str("Files:\n");
    for file in set_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str("\nSTDOUT:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stdout));
    log_content.push_str("\nSTDERR:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stderr));

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("  [ALIGN] Set {}/{}: ✗ Failed!", set_idx + 1, 1);
        return Err(format!("Alignment failed for set {}: {}", set_idx, stderr));
    }

    println!("  [ALIGN] Set {}/{}: ✓ Complete", set_idx + 1, 1);

    // Collect aligned files for this set
    // They will be named align_set_N_0001.tif, align_set_N_0002.tif, etc.
    let mut set_aligned_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(align_folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    // Check if this file belongs to the current set
                    if name_str.starts_with(&format!("align_set_{}_", set_idx)) {
                        if let Some(ext) = path.extension() {
                            if ext.to_string_lossy().to_lowercase() == "tif"
                                || ext.to_string_lossy().to_lowercase() == "tiff"
                            {
                                set_aligned_files.push(path);
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by filename for consistent ordering
    set_aligned_files.sort();
    Ok(set_aligned_files)
}

/// Merge bracketed sets using Blender HDR_Merge.blend
///
/// Command: blender.exe --background HDR_Merge.blend --factory-startup --python blender_merge.py --
///          resolution exr_path filter_used bracket_id imgpath1___ev1 imgpath2___ev2 ...
///
/// # Arguments
/// * `files` - List of aligned file paths to merge
/// * `exr_folder` - Output directory for EXR files (Merged/exr/)
/// * `source_files` - Original scanned files with EXIF data (for file paths)
/// * `ev_source_files` - Files with EXIF data for EV calculation (original RAW files)
/// * `folder` - Folder entry for metadata
/// * `blender_exe` - Path to Blender executable
/// * `logs_dir` - Directory to save log files
/// * `total_sets` - Total number of sets for progress reporting
///
/// # Returns
/// Result indicating success

/// Reload aligned files from the aligned directory
fn reload_aligned_files(align_dir: &Path) -> Result<Vec<crate::scan_folder::ScannedFile>, String> {
    let mut files = Vec::new();

    if !align_dir.exists() {
        return Ok(files);
    }

    if let Ok(entries) = std::fs::read_dir(align_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "tif"
                        || ext.to_string_lossy().to_lowercase() == "tiff"
                    {
                        files.push(crate::scan_folder::ScannedFile {
                            path: path.to_string_lossy().to_string(),
                            exposure_time: None,
                            f_number: None,
                            iso: None,
                        });
                    }
                }
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Align images by bracket set using parallel processing
///
/// # Arguments
/// * `source_files` - List of all source files to align
/// * `align_folder` - Output directory for aligned files (Merged/aligned/)
/// * `use_opencv_align` - Whether to use OpenCV AlignMTB instead of align_image_stack
/// * `align_exe` - Path to align_image_stack executable (used if use_opencv_align is false)
/// * `folder` - Folder entry with bracket/set information
/// * `logs_dir` - Directory to save log files
/// * `threads` - Number of concurrent threads to use
///
/// # Returns
/// List of aligned file paths (all sets combined, in order)
fn align_images_by_set_concurrent(
    source_files: &[PathBuf],
    align_folder: &Path,
    use_opencv_align: bool,
    align_exe: &str,
    folder: &FolderEntry,
    logs_dir: &Path,
    threads: usize,
) -> Result<Vec<PathBuf>, String> {
    if source_files.is_empty() {
        return Ok(Vec::new());
    }

    // Create align output directory
    if let Err(e) = std::fs::create_dir_all(align_folder) {
        return Err(format!("Failed to create align directory: {}", e));
    }

    // Check if align_image_stack is configured (only needed if not using OpenCV)
    if !use_opencv_align && align_exe.is_empty() {
        return Err("align_image_stack not configured in setup".to_string());
    }

    // Group files by bracket sets
    let bracket_count = folder.brackets as usize;
    if bracket_count == 0 {
        return Err("No brackets detected".to_string());
    }

    // Create a vector of set indices to process
    let set_indices: Vec<usize> = (0..folder.sets as usize).collect();

    // Process sets in parallel with limited concurrency
    use rayon::prelude::*;
    let results: Vec<Result<Vec<PathBuf>, String>> = set_indices
        .par_iter()
        .with_max_len(threads)
        .map(|&set_idx| {
            let set_start = Instant::now();
            let start_idx = set_idx * bracket_count;
            let end_idx = std::cmp::min(start_idx + bracket_count, source_files.len());
            let set_files: Vec<PathBuf> = source_files[start_idx..end_idx].to_vec();

            if set_files.len() != bracket_count {
                return Ok(Vec::new());
            }

            if use_opencv_align {
                println!(
                    "  [ALIGN] Set {}/{}: Aligning {} files with OpenCV AlignMTB...",
                    set_idx + 1,
                    folder.sets,
                    set_files.len()
                );
                let aligned = crate::process::opencv_align::align_set_with_opencv(
                    &set_files,
                    align_folder,
                    set_idx,
                    logs_dir,
                )?;
                println!(
                    "  [ALIGN] Set {}/{}: Time: {:.2}s",
                    set_idx + 1,
                    folder.sets,
                    set_start.elapsed().as_secs_f32()
                );
                Ok(aligned)
            } else {
                println!(
                    "  [ALIGN] Set {}/{}: Aligning {} files with align_image_stack...",
                    set_idx + 1,
                    folder.sets,
                    set_files.len()
                );
                let aligned =
                    align_single_set(&set_files, align_folder, set_idx, align_exe, logs_dir)?;
                println!(
                    "  [ALIGN] Set {}/{}: Time: {:.2}s",
                    set_idx + 1,
                    folder.sets,
                    set_start.elapsed().as_secs_f32()
                );
                Ok(aligned)
            }
        })
        .collect();

    // Collect results
    let mut all_aligned_files = Vec::new();
    for result in results {
        match result {
            Ok(files) => all_aligned_files.extend(files),
            Err(e) => return Err(e),
        }
    }

    // Sort by filename to maintain order
    all_aligned_files.sort();

    Ok(all_aligned_files)
}

/// Tone map HDR files to JPG using OpenCV
fn tone_map_with_opencv(
    hdr_folder: &Path,
    jpg_folder: &Path,
    gui_settings: &GuiSettings,
    logs_dir: &Path,
    threads: usize,
) -> Result<(), String> {
    // Get list of HDR files (both EXR and TIFF)
    let mut hdr_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(hdr_folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if ext_lower == "exr" || ext_lower == "tif" || ext_lower == "tiff" {
                        hdr_files.push(path);
                    }
                }
            }
        }
    }

    if hdr_files.is_empty() {
        return Err("No HDR files found for tone mapping".to_string());
    }

    // Sort by filename
    hdr_files.sort();

    // Create tone mapping params from gui_settings
    let operator = match gui_settings.tonemap_operator.to_lowercase().as_str() {
        "drago" => crate::process::opencv_tonemap::ToneMappingOperator::Drago,
        "mantiuk" => crate::process::opencv_tonemap::ToneMappingOperator::Mantiuk,
        _ => crate::process::opencv_tonemap::ToneMappingOperator::Reinhard,
    };

    let params = crate::process::opencv_tonemap::ToneMappingParams {
        operator,
        intensity: gui_settings.tonemap_intensity,
        contrast: gui_settings.tonemap_contrast,
        saturation: gui_settings.tonemap_saturation,
        detail: 0.0,
    };

    // Use OpenCV tone mapping
    crate::process::opencv_tonemap::tone_map_hdr_to_jpg_opencv(
        &hdr_files, jpg_folder, &params, logs_dir, threads,
    )
}

/// Cleanup temporary files
///
/// # Arguments
/// * `merged_dir` - Merged directory path
/// * `aligned` - Whether alignment was performed
///
/// # Returns
/// Result indicating success
fn cleanup_temp_files(merged_dir: &Path, aligned: bool) -> Result<(), String> {
    // Remove aligned directory if it exists
    if aligned {
        let align_dir = merged_dir.join("aligned");
        if align_dir.exists() {
            std::fs::remove_dir_all(&align_dir)
                .map_err(|e| format!("Failed to cleanup aligned files: {}", e))?;
        }
    }

    // Optionally remove tif folder if RAW processing was done
    // This is typically not desired as users may want to keep the TIFFs
    // Could be made configurable in the future

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_process_folder_exists() {
        // Basic test to ensure the function signature is correct
        // Integration tests would require actual files and executables
    }
}
