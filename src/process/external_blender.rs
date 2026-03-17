    //! Blender HDR merging module
//!
//! Handles merging bracketed images into HDR using Blender's HDR Merge addon

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::config::FolderEntry;
use crate::process::ev_calc;

/// Merge bracketed sets using Blender HDR_Merge.blend with parallel processing
pub fn merge_with_blender_concurrent(
    files: &[PathBuf],
    exr_folder: &Path,
    _source_files: &[crate::scan_folder::ScannedFile],
    ev_source_files: &[crate::scan_folder::ScannedFile],
    folder: &FolderEntry,
    blender_exe: &str,
    logs_dir: &Path,
    total_sets: u32,
    threads: usize,
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

    let blender_folder = exe_dir.join("blender");
    let blend_file = blender_folder.join("HDR_Merge.blend");
    let merge_py = blender_folder.join("blender_merge.py");

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

    // Get resolution from the first file
    let resolution = if !files.is_empty() {
        let temp_file = crate::scan_folder::ScannedFile {
            path: files[0].to_string_lossy().to_string(),
            exposure_time: None,
            f_number: None,
            iso: None,
            bias: None,
        };
        get_resolution_from_file(&temp_file)
    } else {
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

    // Create set indices
    let set_indices: Vec<usize> = (0..folder.sets as usize).collect();

    // Process sets in parallel with limited concurrency
    use rayon::prelude::*;
    let results: Vec<Result<(), String>> = set_indices
        .par_iter()
        .with_max_len(threads)
        .map(|&set_idx| {
            let set_start = Instant::now();
            let start_idx = set_idx * bracket_count;
            let end_idx = std::cmp::min(start_idx + bracket_count, files.len());
            let set_files: Vec<PathBuf> = files[start_idx..end_idx].to_vec();

            if set_files.len() != bracket_count {
                return Ok(());
            }

            println!("[BLENDER] Set {}/{}: Merging {} files to HDR...", set_idx + 1, total_sets, set_files.len());

            // Generate output filename
            let exr_filename = format!("merged_{:03}.exr", set_idx);
            let exr_path = exr_folder.join(&exr_filename);

            // Get the corresponding source files for this set (with EXIF data for EV calculation)
            let ev_start = set_idx * bracket_count;
            let ev_end = std::cmp::min(ev_start + bracket_count, ev_source_files.len());
            let set_ev_files: Vec<crate::scan_folder::ScannedFile> = ev_source_files[ev_start..ev_end].to_vec();

            // Calculate relative EV values
            let ev_values = ev_calc::calculate_relative_evs(&set_ev_files);

            println!("    [BLENDER] EV values (brightest=0.0): {:?}", ev_values);
            println!("    [BLENDER] File order (will be sorted by Python):");
            for (i, (file, ev)) in set_files.iter().zip(ev_values.iter()).enumerate() {
                let shutter = set_ev_files.get(i).and_then(|f| f.exposure_time.as_ref()).map(|s| s.as_str()).unwrap_or("N/A");
                println!("      {}: {} (shutter: {}, EV: {:.3})", i, file.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), shutter, ev);
            }

            // Build file list with exposure values
            let mut file_args = Vec::new();
            for (file, ev) in set_files.iter().zip(ev_values.iter()) {
                file_args.push(format!("{}___{:.3}", file.display(), ev));
            }

            // Build Blender command
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

            for file_arg in &file_args {
                cmd.arg(file_arg);
            }

            // Execute Blender
            let output = cmd.output()
                .map_err(|e| format!("Failed to execute Blender: {}", e))?;

            // Save logs
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
                println!("  [BLENDER] Set {}/{}: ✗ Failed!", set_idx + 1, total_sets);
                return Err(format!(
                    "Blender merge failed for set {}: {}\n{}",
                    set_idx, stderr, stdout
                ));
            }

            println!("  [BLENDER] Set {}/{}: ✓ Complete (output: {})", set_idx + 1, total_sets, exr_path.display());
            println!("  [BLENDER] Set {}/{}: Time: {:.2}s", set_idx + 1, total_sets, set_start.elapsed().as_secs_f32());

            Ok(())
        })
        .collect();

    // Check for errors
    for result in results {
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// Get resolution from a file's EXIF data
/// Returns resolution in format "WxH" (e.g., "3456x5184")
fn get_resolution_from_file(file: &crate::scan_folder::ScannedFile) -> String {
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
                return format!("{}x{}", w, h);
            }
        }
    }

    // Fallback
    "0x0".to_string()
}

/// Get filter information from folder profile
fn get_filter_from_profile(folder: &FolderEntry) -> String {
    // Check profile name for filter keywords
    let profile_lower = folder.profile.to_lowercase();

    if profile_lower.contains("nd8") {
        if profile_lower.contains("nd400") {
            "ND8_ND400".to_string()
        } else {
            "ND8".to_string()
        }
    } else if profile_lower.contains("nd400") {
        "ND400".to_string()
    } else {
        String::new()
    }
}
