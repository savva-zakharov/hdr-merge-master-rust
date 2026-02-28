//! HDR Processor implementation
//!
//! Handles the actual processing of bracketed images into HDR
//!
//! Processing Flow:
//! 1. If RAW files: process with RawTherapee CLI → Merged/tif/
//! 2. If align enabled: align with align_image_stack → Merged/aligned/
//! 3. Merge each bracketed set using Blender HDR_Merge.blend → Merged/exr/
//! 4. Tone map EXR to JPG using Luminance CLI → Merged/jpg/
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
    let profile = config.pp3_profiles
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
    println!("[PROCESS] Folders: {} sets, {} brackets per set", folder.sets, folder.brackets);
    println!("[PROCESS] Started at: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));

    // Step 1: If RAW files, process with RawTherapee CLI to Merged/tif/
    let tif_folder = merged_dir.join("tif");
    let step_start = Instant::now();
    let source_files = if folder.is_raw {
        println!("[STEP 1] Processing RAW files with RawTherapee CLI...");
        process_raw_files(&folder.files, &tif_folder, &profile_path, &config.exe_paths.rawtherapee_cli_exe, &logs_dir)?
    } else {
        println!("[STEP 1] Skipping RAW processing (non-RAW files)");
        // For non-RAW files, use them directly
        folder.files.iter().map(|f| PathBuf::from(&f.path)).collect()
    };
    if folder.is_raw {
        println!("[STEP 1] Completed in {:.2}s", step_start.elapsed().as_secs_f32());
    }

    // Step 2: If alignment enabled, align each bracket set with align_image_stack to Merged/aligned/
    let aligned_files = if gui_settings.do_align {
        println!("[STEP 2] Aligning {} bracket sets with align_image_stack...", folder.sets);
        let step_start = Instant::now();
        let align_folder = merged_dir.join("aligned");
        let result = align_images_by_set(&source_files, &align_folder, gui_settings.use_opencv, &config.exe_paths.align_image_stack_exe, folder, &logs_dir)?;
        println!("[STEP 2] Completed in {:.2}s", step_start.elapsed().as_secs_f32());
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
        source_files.iter().map(|p| crate::scan_folder::ScannedFile {
            path: p.to_string_lossy().to_string(),
            exposure_time: None,
            f_number: None,
            iso: None,
        }).collect()
    };
    
    // For EV calculation, we always use the original RAW file data (folder.files)
    // This ensures we have accurate exposure information even after processing
    let ev_source_files = if folder.is_raw {
        // Use original RAW files for EV calculation
        folder.files.clone()
    } else {
        // For non-RAW, use the source files (they have EXIF)
        source_files_for_merge.clone()
    };
    
    println!("[STEP 3] Merging {} bracket sets with Blender...", folder.sets);
    let step_start = Instant::now();
    merge_with_blender(&aligned_files, &exr_folder, &source_files_for_merge, &ev_source_files, folder, &config.exe_paths.blender_exe, &logs_dir, folder.sets)?;
    println!("[STEP 3] Completed in {:.2}s", step_start.elapsed().as_secs_f32());

    // Step 4: Tone map EXR to JPG using Luminance CLI
    let jpg_folder = merged_dir.join("jpg");
    println!("[STEP 4] Tone mapping EXR files to JPG with Luminance CLI...");
    let step_start = Instant::now();
    tone_map_exr_to_jpg(&exr_folder, &jpg_folder, &config.exe_paths.luminance_cli_exe, &logs_dir)?;
    println!("[STEP 4] Completed in {:.2}s", step_start.elapsed().as_secs_f32());

    // Step 5: Cleanup temporary files if enabled
    if gui_settings.do_cleanup {
        println!("[STEP 5] Cleaning up temporary files...");
        cleanup_temp_files(&merged_dir, gui_settings.do_align)?;
    } else {
        println!("[STEP 5] Skipping cleanup (disabled)");
    }

    println!("[PROCESS] ✓ Successfully processed: {}", folder.path);
    let total_elapsed = total_start.elapsed();
    println!("[PROCESS] Total time: {:.2}s ({:.2} min)", total_elapsed.as_secs_f32(), total_elapsed.as_secs_f32() / 60.0);

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
        .arg("-t")  // Use threads
        .arg("-c"); // Overwrite existing files

    // Add all RAW files to process
    for raw_file in raw_files {
        cmd.arg(&raw_file.path);
    }

    println!("  [RAW] Processing {} files...", raw_files.len());
    let step_start = Instant::now();

    // Execute command and capture output
    let output = cmd.output()
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
    
    println!("  [RAW] ✓ Complete (Time: {:.2}s)", step_start.elapsed().as_secs_f32());

    // Collect generated TIFF files
    let mut tif_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(tif_folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "tif" || 
                       ext.to_string_lossy().to_lowercase() == "tiff" {
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

/// Align images by bracket set using either align_image_stack or OpenCV AlignMTB
///
/// Each bracket set is aligned separately to avoid mixing exposures across sets.
///
/// # Arguments
/// * `source_files` - List of all source files to align
/// * `align_folder` - Output directory for aligned files (Merged/aligned/)
/// * `use_opencv` - Whether to use OpenCV AlignMTB instead of align_image_stack
/// * `align_exe` - Path to align_image_stack executable (used if use_opencv is false)
/// * `folder` - Folder entry with bracket/set information
/// * `logs_dir` - Directory to save log files
///
/// # Returns
/// List of aligned file paths (all sets combined, in order)
fn align_images_by_set(
    source_files: &[PathBuf],
    align_folder: &Path,
    use_opencv: bool,
    align_exe: &str,
    folder: &FolderEntry,
    logs_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    if source_files.is_empty() {
        return Ok(Vec::new());
    }

    // Create align output directory
    if let Err(e) = std::fs::create_dir_all(align_folder) {
        return Err(format!("Failed to create align directory: {}", e));
    }

    // Check if align_image_stack is configured (only needed if not using OpenCV)
    if !use_opencv && align_exe.is_empty() {
        return Err("align_image_stack not configured in setup".to_string());
    }

    // Group files by bracket sets
    let bracket_count = folder.brackets as usize;
    if bracket_count == 0 {
        return Err("No brackets detected".to_string());
    }

    let mut all_aligned_files = Vec::new();

    // Process each set separately
    for set_idx in 0..folder.sets as usize {
        let set_start = Instant::now();
        let start_idx = set_idx * bracket_count;
        let end_idx = std::cmp::min(start_idx + bracket_count, source_files.len());
        let set_files = &source_files[start_idx..end_idx];

        if set_files.len() != bracket_count {
            continue;
        }

        if use_opencv {
            println!("  [ALIGN] Set {}/{}: Aligning {} files with OpenCV AlignMTB...", set_idx + 1, folder.sets, set_files.len());
            // Use OpenCV AlignMTB
            let aligned = crate::process::opencv_align::align_set_with_opencv(
                set_files,
                align_folder,
                set_idx,
                logs_dir,
            )?;
            all_aligned_files.extend(aligned);
        } else {
            println!("  [ALIGN] Set {}/{}: Aligning {} files with align_image_stack...", set_idx + 1, folder.sets, set_files.len());
            // Use align_image_stack
            let aligned = align_single_set(
                set_files,
                align_folder,
                set_idx,
                align_exe,
                logs_dir,
            )?;
            all_aligned_files.extend(aligned);
        }
        
        println!("  [ALIGN] Set {}/{}: Time: {:.2}s", set_idx + 1, folder.sets, set_start.elapsed().as_secs_f32());
    }

    Ok(all_aligned_files)
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
    cmd.arg("-v")  // Verbose
        .arg("-i") // Auto-crop
        .arg("-l") // Keep linear values
        .arg("-a")
        .arg(align_folder.join(format!("align_set_{}_", set_idx)).to_str().ok_or("Invalid align folder path")?)
        .arg("--gpu");

    // Add files for this set only
    for file in set_files {
        cmd.arg(file);
    }

    // Execute command and capture output
    let output = cmd.output()
        .map_err(|e| format!("Failed to execute align_image_stack for set {}: {}", set_idx, e))?;

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
                            if ext.to_string_lossy().to_lowercase() == "tif" ||
                               ext.to_string_lossy().to_lowercase() == "tiff" {
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
fn merge_with_blender(
    files: &[PathBuf],
    exr_folder: &Path,
    _source_files: &[crate::scan_folder::ScannedFile],
    ev_source_files: &[crate::scan_folder::ScannedFile],
    folder: &FolderEntry,
    blender_exe: &str,
    logs_dir: &Path,
    total_sets: u32,
) -> Result<(), String> {
    if files.is_empty() {
        return Err("No files to merge".to_string());
    }

    // Check if Blender is configured
    if blender_exe.is_empty() {
        return Err("Blender executable not configured in setup".to_string());
    }

    // Create EXR output directory
    if let Err(e) = std::fs::create_dir_all(exr_folder) {
        return Err(format!("Failed to create exr directory: {}", e));
    }

    // Get the blend file and python script paths
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    
    // Try to find blender folder relative to executable or current directory
    let blender_folder = exe_dir.join("blender");
    let blend_file = blender_folder.join("HDR_Merge.blend");
    let merge_py = blender_folder.join("blender_merge.py");

    // Check if files exist, if not try relative to current directory
    let blend_file = if blend_file.exists() {
        blend_file
    } else {
        let alt_blend = Path::new("blender").join("HDR_Merge.blend");
        if alt_blend.exists() {
            alt_blend
        } else {
            return Err("HDR_Merge.blend not found".to_string());
        }
    };

    let merge_py = if merge_py.exists() {
        merge_py
    } else {
        let alt_py = Path::new("blender").join("blender_merge.py");
        if alt_py.exists() {
            alt_py
        } else {
            return Err("blender_merge.py not found".to_string());
        }
    };

    // Get resolution from the first TIFF file being merged (not the original RAW)
    // The TIFF files have the actual processed resolution
    let resolution = if !files.is_empty() {
        // Create a temporary ScannedFile to read resolution from the first aligned TIFF
        let temp_file = crate::scan_folder::ScannedFile {
            path: files[0].to_string_lossy().to_string(),
            exposure_time: None,
            f_number: None,
            iso: None,
        };
        get_resolution_from_file(&temp_file)
    } else {
        // Fallback to ev_source_files if no files available
        get_resolution_from_file(&ev_source_files[0])
    };
    println!("    [BLENDER] Image resolution: {}", resolution);

    // Get filter information from folder profile
    let filter_used = get_filter_from_profile(folder);

    // Group files by bracket sets
    let bracket_count = folder.brackets as usize;
    if bracket_count == 0 {
        return Err("No brackets detected".to_string());
    }

    // Process each set
    for set_idx in 0..folder.sets as usize {
        let set_start = Instant::now();
        let start_idx = set_idx * bracket_count;
        let end_idx = std::cmp::min(start_idx + bracket_count, files.len());
        let set_files = &files[start_idx..end_idx];

        if set_files.len() != bracket_count {
            continue;
        }

        println!("[BLENDER] Set {}/{}: Merging {} files to HDR...", set_idx + 1, total_sets, set_files.len());

        // Generate output filename
        let exr_filename = format!("merged_{:03}.exr", set_idx);
        let exr_path = exr_folder.join(&exr_filename);

        // Get the corresponding source files for this set (with EXIF data for EV calculation)
        let ev_start = set_idx * bracket_count;
        let ev_end = std::cmp::min(ev_start + bracket_count, ev_source_files.len());
        let set_ev_files = &ev_source_files[ev_start..ev_end];

        // Calculate relative EV values for this set using original RAW file EXIF data
        // The brightest image will have EV=0.0, darker images will have positive EV
        // This matches the Blender Python script's expected format
        let ev_values = crate::process::ev_calc::calculate_relative_evs(set_ev_files);
        
        println!("    [BLENDER] EV values (brightest=0.0): {:?}", ev_values);
        println!("    [BLENDER] File order (will be sorted by Python):");
        for (i, (file, ev)) in set_files.iter().zip(ev_values.iter()).enumerate() {
            let shutter = set_ev_files.get(i).and_then(|f| f.exposure_time.as_ref()).map(|s| s.as_str()).unwrap_or("N/A");
            println!("      {}: {} (shutter: {}, EV: {:.3})", i, file.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), shutter, ev);
        }

        // Build file list with exposure values
        // Format: imgpath1___ev1 imgpath2___ev2 ...
        // Python will sort these by EV value (ascending: 0, 3, 6, 9, 12)
        let mut file_args = Vec::new();
        for (file, ev) in set_files.iter().zip(ev_values.iter()) {
            // Format: filepath___EV (e.g., "align_set_0_0001.tif___0.000")
            file_args.push(format!("{}___{:.3}", file.display(), ev));
        }

        // Build Blender command
        // blender.exe --background HDR_Merge.blend --factory-startup --python blender_merge.py --
        //   resolution exr_path filter_used bracket_id imgpath1___ev1 imgpath2___ev2 ...
        let mut cmd = Command::new(blender_exe);
        cmd.arg("--background")
            .arg(&blend_file)
            .arg("--factory-startup")
            .arg("--python")
            .arg(&merge_py)
            .arg("--")
            .arg(&resolution)
            .arg(exr_path.to_str().ok_or("Invalid EXR output path")?)
            .arg(&filter_used)
            .arg(set_idx.to_string());

        // Add file arguments
        for file_arg in &file_args {
            cmd.arg(file_arg);
        }

        // Execute Blender and capture output
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute Blender: {}", e))?;

        // Save logs for this set
        let log_file = logs_dir.join(format!("blender_merge_set_{:03}.log", set_idx));
        let mut log_content = String::new();
        log_content.push_str(&format!("=== Blender HDR Merge - Set {} ===\n\n", set_idx));
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
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("  [BLENDER] Set {}/{}: ✗ Failed!", set_idx + 1, folder.sets);
            return Err(format!(
                "Blender merge failed for set {}: {}\n{}",
                set_idx, stderr, stdout
            ));
        }
        
        println!("  [BLENDER] Set {}/{}: ✓ Complete (output: {})", set_idx + 1, total_sets, exr_path.display());
        println!("  [BLENDER] Set {}/{}: Time: {:.2}s", set_idx + 1, total_sets, set_start.elapsed().as_secs_f32());
    }

    Ok(())
}

/// Get resolution from a file's EXIF data
/// Returns resolution in format "WxH" (e.g., "3456x5184")
fn get_resolution_from_file(file: &crate::scan_folder::ScannedFile) -> String {
    // Try to read resolution from EXIF data
    // Try different possible EXIF tag names for image dimensions
    use exif::{In, Tag};
    use std::fs::File;
    use std::io::BufReader;
    
    if let Ok(file_handle) = File::open(&file.path) {
        let mut bufreader = BufReader::new(&file_handle);
        let exif_reader = exif::Reader::new();
        
        if let Ok(exif) = exif_reader.read_from_container(&mut bufreader) {
            // Try 1: Image ImageWidth / ImageLength (PRIMARY IFD)
            let width = exif.get_field(Tag::ImageWidth, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                .map(|v| v as u32);
            
            let height = exif.get_field(Tag::ImageLength, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                .map(|v| v as u32);
            
            if let (Some(w), Some(h)) = (width, height) {
                println!("    [BLENDER] Resolution from ImageWidth/ImageLength: {}x{}", w, h);
                return format!("{}x{}", w, h);
            }
            
            // Try 2: PixelXDimension / PixelYDimension (PRIMARY IFD)
            let width = exif.get_field(Tag::PixelXDimension, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                .map(|v| v as u32);
            
            let height = exif.get_field(Tag::PixelYDimension, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                .map(|v| v as u32);
            
            if let (Some(w), Some(h)) = (width, height) {
                println!("    [BLENDER] Resolution from PixelXDimension/PixelYDimension: {}x{}", w, h);
                return format!("{}x{}", w, h);
            }
            
            // Try 3: Search all fields for dimension tags
            let mut width: Option<u32> = None;
            let mut height: Option<u32> = None;
            
            for field in exif.fields() {
                if let Some(val) = field.value.get_uint(0) {
                    let tag_name = format!("{:?}", field.tag);
                    if val > 0 && val < 100000 {
                        if tag_name.contains("Width") || tag_name.contains("XDimension") {
                            width = Some(val);
                        } else if tag_name.contains("Length") || tag_name.contains("YDimension") {
                            height = Some(val);
                        }
                    }
                }
            }
            
            if let (Some(w), Some(h)) = (width, height) {
                println!("    [BLENDER] Resolution from field search: {}x{}", w, h);
                return format!("{}x{}", w, h);
            }
        }
    }
    
    // Fallback: try to read image dimensions using image crate
    if let Ok(dimensions) = image::image_dimensions(&file.path) {
        println!("    [BLENDER] Resolution from image crate: {}x{}", dimensions.0, dimensions.1);
        return format!("{}x{}", dimensions.0, dimensions.1);
    }
    
    println!("    [BLENDER] Resolution: using default 3456x5184 (could not read from EXIF)");
    // Last resort: return default resolution
    "3456x5184".to_string()
}

/// Get filter information from profile tag
/// Returns filter string like "ND8_ND400" or empty if no filter
fn get_filter_from_profile(_folder: &FolderEntry) -> String {
    // The profile tag field can contain filter information
    // For now, return empty - this could be parsed from folder.profile or config
    String::new()
}

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
                    if ext.to_string_lossy().to_lowercase() == "tif" || 
                       ext.to_string_lossy().to_lowercase() == "tiff" {
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

/// Tone map EXR files to JPG using Luminance CLI
///
/// Command: luminance_cli_exe -l exr_path --tmo reinhard02 -q 98 -o jpg_path
///
/// # Arguments
/// * `exr_folder` - Input directory containing EXR files
/// * `jpg_folder` - Output directory for JPG files (Merged/jpg/)
/// * `luminance_exe` - Path to Luminance CLI executable
/// * `logs_dir` - Directory to save log files
///
/// # Returns
/// Result indicating success
fn tone_map_exr_to_jpg(
    exr_folder: &Path,
    jpg_folder: &Path,
    luminance_exe: &str,
    logs_dir: &Path,
) -> Result<(), String> {
    // Create JPG output directory
    if let Err(e) = std::fs::create_dir_all(jpg_folder) {
        return Err(format!("Failed to create jpg directory: {}", e));
    }

    // Check if Luminance CLI is configured
    if luminance_exe.is_empty() {
        return Err("Luminance CLI not configured in setup".to_string());
    }

    // Get all EXR files from the exr folder
    let mut exr_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(exr_folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "exr" {
                        exr_files.push(path);
                    }
                }
            }
        }
    }

    // Sort by filename for consistent ordering
    exr_files.sort();

    if exr_files.is_empty() {
        return Err("No EXR files found to tone map".to_string());
    }

    // Process each EXR file
    for (idx, exr_path) in exr_files.iter().enumerate() {
        let file_start = Instant::now();
        let filename = exr_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
        println!("[TONEMAP] File {}/{}: {}", idx + 1, exr_files.len(), filename);
        
        // Generate output JPG filename
        let jpg_filename = exr_path
            .file_stem()
            .map(|s| format!("{}.jpg", s.to_string_lossy()))
            .ok_or("Invalid EXR filename")?;
        
        let jpg_path = jpg_folder.join(&jpg_filename);

        // Build Luminance CLI command
        // luminance_cli_exe -l exr_path --tmo reinhard02 -q 98 -o jpg_path
        let mut cmd = Command::new(luminance_exe);
        cmd.arg("-l")
            .arg(exr_path.to_str().ok_or("Invalid EXR path")?)
            .arg("--tmo")
            .arg("reinhard02")
            .arg("-q")
            .arg("98")
            .arg("-o")
            .arg(jpg_path.to_str().ok_or("Invalid JPG output path")?);

        // Execute command and capture output
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute Luminance CLI: {}", e))?;

        // Save logs for this file
        let log_filename = exr_path
            .file_stem()
            .map(|s| format!("luminance_{}.log", s.to_string_lossy()))
            .unwrap_or_else(|| "luminance.log".to_string());
        let log_file = logs_dir.join(log_filename);
        
        let mut log_content = String::new();
        log_content.push_str("=== Luminance CLI Tone Mapping ===\n\n");
        log_content.push_str(&format!("Input: {}\n", exr_path.display()));
        log_content.push_str(&format!("Output: {}\n", jpg_path.display()));
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
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("  [TONEMAP] File {}: ✗ Failed", exr_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default());
            return Err(format!(
                "Tone mapping failed for {}: {}\n{}",
                exr_path.display(), stderr, stdout
            ));
        }
        
        println!("  [TONEMAP] File {}: ✓ Complete", filename);
        println!("  [TONEMAP] File {}: Time: {:.2}s", filename, file_start.elapsed().as_secs_f32());
    }

    Ok(())
}

/// Cleanup temporary files
///
/// # Arguments
/// * `merged_dir` - Merged directory path
/// * `aligned` - Whether alignment was performed
///
/// # Returns
/// Result indicating success
fn cleanup_temp_files(
    merged_dir: &Path,
    aligned: bool,
) -> Result<(), String> {
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
