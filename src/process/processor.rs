//! HDR Processor implementation
//!
//! Handles the actual processing of bracketed images into HDR
//!
//! Processing Flow (per bracket set):
//! 1. If RAW files: process with RawTherapee CLI → memory
//! 2. If align enabled: align with align_image_stack/OpenCV → memory
//! 3. Merge bracket set using Blender/OpenCV/Rust → memory
//! 4. Tone map to JPG → save to disk (in parallel with next bracket)
//!
//! All steps log output to Merged/logs/

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Sender, Receiver};
use std::time::Instant;

use crate::config::{Config, FolderEntry, GuiSettings};

/// Message for disk I/O operations
#[derive(Debug, Clone)]
enum IoMessage {
    SaveJpg {
        jpg_data: Vec<u8>,
        output_path: PathBuf,
        set_idx: usize,
    },
    Shutdown,
}

/// I/O worker that handles disk operations in parallel
struct IoWorker {
    sender: Sender<IoMessage>,
}

impl IoWorker {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        
        // Spawn I/O thread
        std::thread::spawn(move || {
            Self::io_loop(receiver);
        });
        
        Self { sender }
    }
    
    fn io_loop(receiver: Receiver<IoMessage>) {
        while let Ok(msg) = receiver.recv() {
            match msg {
                IoMessage::SaveJpg { jpg_data, output_path, set_idx } => {
                    // Ensure parent directory exists
                    if let Some(parent) = output_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    
                    // Save file
                    match std::fs::write(&output_path, jpg_data) {
                        Ok(_) => {
                            println!("  [IO] Set {}: Saved {}", set_idx + 1, output_path.display());
                        }
                        Err(e) => {
                            eprintln!("  [IO] Set {}: Failed to save {}: {}", set_idx + 1, output_path.display(), e);
                        }
                    }
                }
                IoMessage::Shutdown => {
                    break;
                }
            }
        }
    }
    
    fn save_jpg(&self, jpg_data: Vec<u8>, output_path: PathBuf, set_idx: usize) -> Result<(), String> {
        self.sender.send(IoMessage::SaveJpg { jpg_data, output_path, set_idx })
            .map_err(|e| format!("Failed to queue I/O operation: {}", e))
    }
    
    fn shutdown(self) {
        let _ = self.sender.send(IoMessage::Shutdown);
    }
}

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

    // Create output directories
    let jpg_folder = merged_dir.join("jpg");
    if let Err(e) = std::fs::create_dir_all(&jpg_folder) {
        return Err(format!("Failed to create jpg directory: {}", e));
    }

    let total_start = Instant::now();
    println!("[PROCESS] Starting HDR processing for: {}", folder.path);
    println!(
        "[PROCESS] Total: {} sets, {} brackets per set, {} threads",
        folder.sets, folder.brackets, gui_settings.threads
    );
    println!(
        "[PROCESS] Started at: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // Start I/O worker for parallel disk operations
    let io_worker = IoWorker::new();

    // Process each bracket set
    let mut completed_sets = 0u32;
    let mut errors = Vec::new();

    // Group files by sets
    let bracket_count = folder.brackets as usize;
    let total_sets = folder.sets as usize;

    for set_idx in 0..total_sets {
        let set_start = Instant::now();
        let start_idx = set_idx * bracket_count;
        let end_idx = std::cmp::min(start_idx + bracket_count, folder.files.len());
        let set_files: Vec<&crate::scan_folder::ScannedFile> = folder.files[start_idx..end_idx].iter().collect();

        if set_files.len() != bracket_count {
            errors.push(format!("Set {} has incorrect number of files", set_idx + 1));
            continue;
        }

        println!(
            "\n[SET {}/{}] Processing {} files...",
            set_idx + 1, total_sets, set_files.len()
        );

        // Process this bracket set
        match process_single_set(
            &set_files,
            set_idx,
            &merged_dir,
            &jpg_folder,
            &logs_dir,
            &profile_path,
            config,
            gui_settings,
            &io_worker,
        ) {
            Ok(_) => {
                completed_sets += 1;
                println!(
                    "[SET {}/{}] ✓ Complete in {:.2}s",
                    set_idx + 1, total_sets, set_start.elapsed().as_secs_f32()
                );
            }
            Err(e) => {
                errors.push(format!("Set {}: {}", set_idx + 1, e));
                println!("  [SET {}/{}] ✗ Failed: {}", set_idx + 1, total_sets, e);
            }
        }
    }

    // Wait for all I/O operations to complete
    io_worker.shutdown();
    // Give I/O thread time to finish
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Cleanup if enabled
    if gui_settings.do_cleanup {
        println!("\n[CLEANUP] Cleaning up temporary files...");
        cleanup_temp_files(&merged_dir, gui_settings.do_align)?;
    }

    println!("\n[PROCESS] {} Successfully processed: {}", 
        if errors.is_empty() { "✓" } else { "⚠" }, folder.path);
    let total_elapsed = total_start.elapsed();
    println!(
        "[PROCESS] Total time: {:.2}s ({:.2} min)",
        total_elapsed.as_secs_f32(),
        total_elapsed.as_secs_f32() / 60.0
    );
    println!(
        "[PROCESS] Completed {}/{} sets",
        completed_sets, total_sets
    );

    if errors.is_empty() {
        Ok(format!("Successfully processed {} sets in {:.2}s", 
            completed_sets, total_elapsed.as_secs_f32()))
    } else {
        Err(format!(
            "Completed {}/{} sets with {} errors: {}",
            completed_sets, total_sets, errors.len(), errors.join("; ")
        ))
    }
}

/// Process a single bracket set
///
/// Flow:
/// 1. RAW processing (if needed) → memory/buffer
/// 2. Alignment (if enabled) → memory/buffer
/// 3. HDR Merge → memory/buffer
/// 4. Tone mapping → I/O worker (parallel)
fn process_single_set(
    set_files: &[&crate::scan_folder::ScannedFile],
    set_idx: usize,
    merged_dir: &Path,
    jpg_folder: &Path,
    logs_dir: &Path,
    profile_path: &Option<String>,
    config: &Config,
    gui_settings: &GuiSettings,
    io_worker: &IoWorker,
) -> Result<(), String> {
    let mut current_files: Vec<PathBuf>;
    let mut temp_files_to_cleanup: Vec<PathBuf> = Vec::new();

    // Step 1: RAW processing (if needed)
    let is_raw = set_files.iter().any(|f| {
        Path::new(&f.path).extension()
            .map(|ext| {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                matches!(ext_lower.as_str(), "dng" | "cr2" | "cr3" | "nef" | "arw" | "raf" | "orf" | "rw2" | "pef")
            })
            .unwrap_or(false)
    });

    if is_raw {
        println!("  [STEP 1] Processing {} RAW files...", set_files.len());
        let step_start = Instant::now();
        
        let tif_folder = merged_dir.join("tif");
        current_files = process_raw_files_for_set(
            set_files,
            &tif_folder,
            profile_path,
            &config.exe_paths.rawtherapee_cli_exe,
            logs_dir,
            set_idx,
        )?;
        temp_files_to_cleanup.extend(current_files.clone());
        
        println!("    Time: {:.2}s", step_start.elapsed().as_secs_f32());
    } else {
        println!("  [STEP 1] Skipping RAW (non-RAW files)");
        current_files = set_files.iter().map(|f| PathBuf::from(&f.path)).collect();
    }

    // Step 2: Alignment (if enabled)
    if gui_settings.do_align {
        println!("  [STEP 2] Aligning {} files...", current_files.len());
        let step_start = Instant::now();
        
        let align_folder = merged_dir.join("aligned");
        let use_opencv = gui_settings.use_opencv_align;
        let empty_string = String::new();
        
        current_files = align_single_set(
            &current_files,
            &align_folder,
            set_idx,
            if use_opencv { &empty_string } else { &config.exe_paths.align_image_stack_exe },
            logs_dir,
            use_opencv,
        )?;
        // Don't add to temp_files - alignment files are cleaned up separately
        
        println!("    Time: {:.2}s", step_start.elapsed().as_secs_f32());
    } else {
        println!("  [STEP 2] Skipping alignment");
    }

    // Step 3: HDR Merge
    println!("  [STEP 3] Merging HDR...");
    let step_start = Instant::now();
    
    let exr_folder = merged_dir.join("exr");
    
    // Get EV values from original files
    let ev_source_files: Vec<crate::scan_folder::ScannedFile> = set_files.iter().map(|f| (*f).clone()).collect();
    
    let hdr_output_path = if gui_settings.use_opencv_debevec {
        crate::process::opencv_merge::merge_single_set_debevec(
            &current_files,
            &exr_folder,
            &ev_source_files,
            set_idx,
            logs_dir,
        )?
    } else if gui_settings.use_opencv_merge_robertson {
        crate::process::opencv_merge::merge_single_set_robertson(
            &current_files,
            &exr_folder,
            &ev_source_files,
            set_idx,
            logs_dir,
        )?
    } else if gui_settings.use_rust_merge {
        let debug_export_path = if gui_settings.rust_merge_debug_export {
            let debug_dir = merged_dir.join("debug_rust_merge");
            let _ = std::fs::create_dir_all(&debug_dir);
            Some(debug_dir)
        } else {
            None
        };
        
        crate::process::rust_merge::merge_single_set(
            &current_files,
            &exr_folder,
            &ev_source_files,
            set_idx,
            logs_dir,
            debug_export_path.as_deref(),
        )?
    } else {
        // Blender merge
        crate::process::external_blender::merge_single_set(
            &current_files,
            &exr_folder,
            &ev_source_files,
            set_idx,
            &config.exe_paths.blender_exe,
            logs_dir,
        )?
    };
    
    println!("    Time: {:.2}s", step_start.elapsed().as_secs_f32());

    // Step 4: Tone mapping
    println!("  [STEP 4] Tone mapping to JPG...");
    let step_start = Instant::now();
    
    let jpg_output_path = jpg_folder.join(format!("HDR_set_{:03}.jpg", set_idx));
    
    if gui_settings.use_opencv_tonemap {
        // OpenCV tone mapping - reads from EXR, outputs JPG data
        let jpg_data = crate::process::opencv_tonemap::tone_map_single_file_opencv(
            &hdr_output_path,
            gui_settings,
        )?;
        
        // Queue I/O operation (parallel with next bracket processing)
        io_worker.save_jpg(jpg_data, jpg_output_path, set_idx)?;
    } else {
        // Luminance CLI tone mapping
        crate::process::external_luminance::tone_map_single_file(
            &hdr_output_path,
            &jpg_output_path,
            &config.exe_paths.luminance_cli_exe,
            logs_dir,
            set_idx,
        )?;
    }
    
    println!("    Time: {:.2}s", step_start.elapsed().as_secs_f32());

    // Cleanup temporary RAW files if any
    for temp_file in &temp_files_to_cleanup {
        let _ = std::fs::remove_file(temp_file);
    }

    Ok(())
}

/// Process RAW files for a single set
fn process_raw_files_for_set(
    set_files: &[&crate::scan_folder::ScannedFile],
    tif_folder: &Path,
    profile_path: &Option<String>,
    rawtherapee_exe: &str,
    logs_dir: &Path,
    set_idx: usize,
) -> Result<Vec<PathBuf>, String> {
    if let Err(e) = std::fs::create_dir_all(tif_folder) {
        return Err(format!("Failed to create tif directory: {}", e));
    }

    let Some(pp3_file) = profile_path else {
        return Err("No PP3 profile selected for RAW processing".to_string());
    };

    if rawtherapee_exe.is_empty() {
        return Err("RawTherapee CLI not configured".to_string());
    }

    let mut cmd = Command::new(rawtherapee_exe);
    cmd.arg("-p")
        .arg(pp3_file)
        .arg("-o")
        .arg(tif_folder.to_str().ok_or("Invalid tif folder path")?)
        .arg("-t")
        .arg("-c");

    for raw_file in set_files {
        cmd.arg(&raw_file.path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute RawTherapee CLI: {}", e))?;

    // Save logs
    let log_file = logs_dir.join(format!("rawtherapee_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== RawTherapee CLI - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Command: {:?}\n\n", cmd));
    log_content.push_str("STDOUT:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stdout));
    log_content.push_str("\nSTDERR:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stderr));
    let _ = std::fs::write(&log_file, &log_content);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("RawTherapee processing failed: {}", stderr));
    }

    // Collect generated TIFF files
    let mut tif_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(tif_folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if ext_lower == "tif" || ext_lower == "tiff" {
                        // Check if this file was just created for this set
                        if set_files.iter().any(|f| {
                            let raw_name = Path::new(&f.path).file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let tif_name = path.file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            raw_name == tif_name
                        }) {
                            tif_files.push(path);
                        }
                    }
                }
            }
        }
    }

    tif_files.sort();
    Ok(tif_files)
}

/// Align a single bracket set
fn align_single_set(
    set_files: &[PathBuf],
    align_folder: &Path,
    set_idx: usize,
    align_exe: &str,
    logs_dir: &Path,
    use_opencv: bool,
) -> Result<Vec<PathBuf>, String> {
    if let Err(e) = std::fs::create_dir_all(align_folder) {
        return Err(format!("Failed to create align directory: {}", e));
    }

    if !use_opencv && align_exe.is_empty() {
        return Err("align_image_stack not configured".to_string());
    }

    if use_opencv {
        crate::process::opencv_align::align_set_with_opencv(
            set_files,
            align_folder,
            set_idx,
            logs_dir,
        )
    } else {
        // align_image_stack
        let mut cmd = Command::new(align_exe);
        cmd.arg("-v")
            .arg("-i")
            .arg("-l")
            .arg("-a")
            .arg(
                align_folder
                    .join(format!("align_set_{}_", set_idx))
                    .to_str()
                    .ok_or("Invalid align folder path")?,
            )
            .arg("--gpu");

        for file in set_files {
            cmd.arg(file);
        }

        let output = cmd.output().map_err(|e| {
            format!("Failed to execute align_image_stack: {}", e)
        })?;

        // Save logs
        let log_file = logs_dir.join(format!("align_set_{:03}.log", set_idx));
        let mut log_content = String::new();
        log_content.push_str(&format!("=== Align Image Stack - Set {} ===\n\n", set_idx));
        log_content.push_str(&format!("Command: {:?}\n\n", cmd));
        log_content.push_str("STDOUT:\n");
        log_content.push_str(&String::from_utf8_lossy(&output.stdout));
        log_content.push_str("\nSTDERR:\n");
        log_content.push_str(&String::from_utf8_lossy(&output.stderr));
        let _ = std::fs::write(&log_file, &log_content);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Alignment failed: {}", stderr));
        }

        // Collect aligned files
        let mut aligned_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(align_folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with(&format!("align_set_{}_", set_idx)) {
                            if let Some(ext) = path.extension() {
                                let ext_lower = ext.to_string_lossy().to_lowercase();
                                if ext_lower == "tif" || ext_lower == "tiff" {
                                    aligned_files.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        aligned_files.sort();
        Ok(aligned_files)
    }
}

/// Cleanup temporary files
fn cleanup_temp_files(merged_dir: &Path, aligned: bool) -> Result<(), String> {
    if aligned {
        let align_dir = merged_dir.join("aligned");
        if align_dir.exists() {
            std::fs::remove_dir_all(&align_dir)
                .map_err(|e| format!("Failed to cleanup aligned files: {}", e))?;
        }
    }

    Ok(())
}
