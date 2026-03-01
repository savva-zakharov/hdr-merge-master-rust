//! OpenCV-based HDR merging using MergeDebevec algorithm
//!
//! This module provides an alternative to Blender for HDR creation using
//! OpenCV's MergeDebevec algorithm with CalibrateDebevec for camera response.

use std::path::{Path, PathBuf};
use std::time::Instant;

use opencv::{
    prelude::*,
    photo::{create_calibrate_debevec, create_merge_debevec},
    imgcodecs::{imread, imwrite, IMREAD_COLOR},
    core::{Vector, Mat},
};

/// Merge bracketed images into HDR using OpenCV's MergeDebevec algorithm
///
/// # Arguments
/// * `source_files` - List of bracketed image paths (different exposures)
/// * `exposure_times` - Exposure times in seconds for each image
/// * `output_path` - Output EXR file path
/// * `logs_dir` - Directory to save log files
/// * `set_idx` - Set index for logging
///
/// # Returns
/// Path to the generated HDR EXR file
pub fn merge_with_debevec(
    source_files: &[PathBuf],
    exposure_times: &[f32],
    output_path: &Path,
    logs_dir: &Path,
    set_idx: usize,
) -> Result<PathBuf, String> {
    let merge_start = Instant::now();

    println!("    [OPENCV-MERGE] Merging {} files with MergeDebevec...", source_files.len());

    if source_files.len() != exposure_times.len() {
        return Err(format!(
            "Number of files ({}) must match number of exposure times ({})",
            source_files.len(),
            exposure_times.len()
        ));
    }

    // Load all images
    let mut images: Vector<Mat> = Vector::new();
    for src_file in source_files {
        let img = imread(&src_file.to_string_lossy(), IMREAD_COLOR)
            .map_err(|e| format!("Failed to load {}: {}", src_file.display(), e))?;
        if img.empty() {
            return Err(format!("Loaded empty image from {}", src_file.display()));
        }
        images.push(img);
    }

    if images.is_empty() {
        return Err("No images to merge".to_string());
    }

    // Create exposure times vector
    let mut times: Vector<f32> = Vector::new();
    for &t in exposure_times {
        times.push(t);
    }

    // Calibrate camera response using CalibrateDebevec
    // Parameters: samples (number of pixel samples), lambda (smoothness), random (random sampling)
    let mut calibrate = create_calibrate_debevec(100, 1.0, false)
        .map_err(|e| format!("Failed to create CalibrateDebevec: {}", e))?;
    
    let mut response: Mat = Mat::default();
    calibrate.process(&images, &mut response, &times)
        .map_err(|e| format!("CalibrateDebevec failed: {}", e))?;

    println!("    [OPENCV-MERGE] Camera response calibrated");

    // Merge images using MergeDebevec
    let mut merge = create_merge_debevec()
        .map_err(|e| format!("Failed to create MergeDebevec: {}", e))?;
    
    let mut hdr: Mat = Mat::default();
    opencv::photo::MergeDebevecTrait::process(&mut merge, &images, &mut hdr, &times)
        .map_err(|e| format!("MergeDebevec failed: {}", e))?;

    println!("    [OPENCV-MERGE] HDR image created");

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Save HDR as 32-bit float EXR (standard HDR format)
    // Note: Requires OpenCV to be built with EXR support (OPENCV_IO_ENABLE_OPENEXR)
    let params: Vector<i32> = vec![
        opencv::imgcodecs::IMWRITE_EXR_TYPE as i32,
        opencv::imgcodecs::IMWRITE_EXR_TYPE_FLOAT as i32,  // 32-bit float
    ].into_iter().collect();

    // Try to save as EXR first
    let saved_path = match imwrite(&output_path.with_extension("exr").to_string_lossy(), &hdr, &params) {
        Ok(_) => {
            println!("    [OPENCV-MERGE] Saved HDR EXR to {}", output_path.with_extension("exr").display());
            output_path.with_extension("exr")
        }
        Err(e) => {
            // EXR not supported, save as 16-bit TIFF fallback
            println!("    [OPENCV-MERGE] EXR not supported ({}), saving as TIFF", e);
            let tiff_params: Vector<i32> = vec![
                opencv::imgcodecs::IMWRITE_TIFF_COMPRESSION as i32,
                1,  // No compression
            ].into_iter().collect();
            
            imwrite(&output_path.to_string_lossy(), &hdr, &tiff_params)
                .map_err(|e| format!("Failed to save HDR TIFF: {}", e))?;
            println!("    [OPENCV-MERGE] Saved HDR TIFF to {}", output_path.display());
            output_path.to_path_buf()
        }
    };

    // Create log entry
    let log_file = logs_dir.join(format!("opencv_merge_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== OpenCV HDR Merge (MergeDebevec) - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Input files: {}\n", source_files.len()));
    for (file, exp_time) in source_files.iter().zip(exposure_times.iter()) {
        log_content.push_str(&format!("  {} (exposure: {}s)\n", file.display(), exp_time));
    }
    log_content.push_str(&format!("\nOutput file: {}\n", saved_path.display()));
    log_content.push_str(&format!("\nProcessing time: {:.2}s\n", merge_start.elapsed().as_secs_f32()));
    log_content.push_str("\n✓ HDR merge completed using OpenCV MergeDebevec algorithm.\n");

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    println!("    [OPENCV-MERGE] Set {}: ✓ Complete (Time: {:.2}s)",
        set_idx, merge_start.elapsed().as_secs_f32());

    Ok(saved_path)
}

/// Merge bracketed sets using OpenCV MergeDebevec with parallel processing
pub fn merge_with_opencv_debevec_concurrent(
    files: &[PathBuf],
    exr_folder: &Path,
    ev_source_files: &[crate::scan_folder::ScannedFile],
    folder: &crate::config::FolderEntry,
    logs_dir: &Path,
    total_sets: u32,
    threads: usize,
) -> Result<(), String> {
    if files.is_empty() {
        return Err("No files to merge".to_string());
    }

    // Create EXR output directory
    if let Err(e) = std::fs::create_dir_all(exr_folder) {
        return Err(format!("Failed to create exr directory: {}", e));
    }

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

            println!("[OPENCV-MERGE] Set {}/{}: Merging {} files to HDR...", set_idx + 1, total_sets, set_files.len());

            // Generate output filename
            let out_filename = format!("merged_{:03}.exr", set_idx);
            let exr_path = exr_folder.join(&out_filename);

            // Get the corresponding source files for this set (with EXIF data for exposure times)
            let ev_start = set_idx * bracket_count;
            let ev_end = std::cmp::min(ev_start + bracket_count, ev_source_files.len());
            let set_ev_files: Vec<crate::scan_folder::ScannedFile> = ev_source_files[ev_start..ev_end].to_vec();

            // Extract exposure times
            let exposure_times = crate::process::opencv_merge::extract_exposure_times(&set_ev_files);

            println!("    [OPENCV-MERGE] Exposure times: {:?}", exposure_times);
            println!("    [OPENCV-MERGE] File order:");
            for (i, file) in set_files.iter().enumerate() {
                println!("      {}: {} (exposure: {}s)", i, file.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), exposure_times.get(i).unwrap_or(&0.01));
            }

            // Merge using OpenCV MergeDebevec
            let result_path = crate::process::opencv_merge::merge_with_debevec(
                &set_files,
                &exposure_times,
                &exr_path,
                logs_dir,
                set_idx,
            )?;

            println!("  [OPENCV-MERGE] Set {}/{}: ✓ Complete (output: {})", set_idx + 1, total_sets, result_path.display());
            println!("  [OPENCV-MERGE] Set {}/{}: Time: {:.2}s", set_idx + 1, total_sets, set_start.elapsed().as_secs_f32());

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

/// Parse exposure time string to f32 (in seconds)
fn parse_exposure_time(exp_str: &str) -> Option<f32> {
    let exp_str = exp_str.trim();

    if exp_str.contains('/') {
        // Fraction format: "1/100"
        let parts: Vec<&str> = exp_str.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(denom)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                if denom != 0.0 {
                    return Some(num / denom);
                }
            }
        }
    } else {
        // Decimal format: "0.5" or "2"
        if let Ok(val) = exp_str.parse::<f32>() {
            return Some(val);
        }
    }

    None
}

/// Extract exposure times from scanned files
///
/// # Arguments
/// * `files` - Slice of ScannedFile with exposure_time data
///
/// # Returns
/// Vector of exposure times in seconds
pub fn extract_exposure_times(files: &[crate::scan_folder::ScannedFile]) -> Vec<f32> {
    files.iter()
        .map(|f| {
            f.exposure_time
                .as_ref()
                .and_then(|s| parse_exposure_time(s))
                .unwrap_or(0.01)  // Default to 1/100s if not available
        })
        .collect()
}
