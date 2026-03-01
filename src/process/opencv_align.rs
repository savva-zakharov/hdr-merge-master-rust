//! OpenCV-based image alignment using AlignMTB
//!
//! This module provides an alternative to align_image_stack using OpenCV's
//! AlignMTB (Median Threshold Bitmap) algorithm, which is specifically designed
//! for aligning exposure-bracketed images for HDR processing.
//!
//! **NOTE**: This module requires OpenCV to be installed on your system.
//! See OPENCV_SETUP.md for installation instructions.
//!
//! **OpenCV Requirements**:
//! - OpenCV 4.x with the photo module
//! - The `createAlignMTB()` and `CalibrateCRF` functions from photo module

use std::path::{Path, PathBuf};
use std::time::Instant;

use opencv::{
    prelude::*,
    photo::create_align_mtb,
    imgcodecs::{imread, imwrite, IMREAD_COLOR, IMREAD_GRAYSCALE, IMREAD_UNCHANGED, IMWRITE_TIFF_COMPRESSION},
    core::{Vector, Mat},
    imgproc,
};

/// Align images using OpenCV's AlignMTB algorithm
///
/// AlignMTB uses Median Threshold Bitmaps which are robust to exposure changes,
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

    println!("    [OPENCV] Aligning {} files with AlignMTB...", source_files.len());

    // Create align output directory
    if let Err(e) = std::fs::create_dir_all(align_folder) {
        return Err(format!("Failed to create align directory: {}", e));
    }

    // Load all images as OpenCV Mats (grayscale for AlignMTB)
    let mut gray_images: Vector<Mat> = Vector::new();
    let mut color_images: Vector<Mat> = Vector::new();
    for src_file in source_files {
        // AlignMTB requires grayscale images for shift calculation
        // Use IMREAD_GRAYSCALE - this will preserve bit depth (8 or 16 bit)
        let gray = imread(&src_file.to_string_lossy(), IMREAD_GRAYSCALE)
            .map_err(|e| format!("Failed to load {}: {}", src_file.display(), e))?;
        if gray.empty() {
            return Err(format!("Loaded empty image from {}", src_file.display()));
        }
        gray_images.push(gray);

        // Load color version with original bit depth preserved
        // IMREAD_UNCHANGED preserves the original bit depth (8 or 16 bit)
        let color = imread(&src_file.to_string_lossy(), IMREAD_UNCHANGED)
            .map_err(|e| format!("Failed to load color {}: {}", src_file.display(), e))?;
        if color.empty() {
            return Err(format!("Loaded empty color image from {}", src_file.display()));
        }
        
        // Convert grayscale to BGR if needed (IMREAD_UNCHANGED might load as grayscale)
        if color.channels() == 1 {
            let mut color_bgr = Mat::default();
            imgproc::cvt_color(&color, &mut color_bgr, imgproc::COLOR_GRAY2BGR, 0, opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT)
                .map_err(|e| format!("Failed to convert to BGR: {}", e))?;
            color_images.push(color_bgr);
        } else {
            color_images.push(color);
        }
    }

    if gray_images.is_empty() {
        return Err("No images to align".to_string());
    }

    // Create AlignMTB instance
    let mut align_mtb = create_align_mtb(
        6,      // max_bits - number of bits used in MTB calculation
        1,      // exclude_range - how many levels to exclude from MTB
        false,  // cut - whether to cut the image borders
    ).map_err(|e| format!("Failed to create AlignMTB: {}", e))?;

    // Calculate shifts between images (comparing each to the first/reference image)
    let mut shifts: Vector<opencv::core::Point> = Vector::new();
    let reference = gray_images.get(0).unwrap();
    
    // First shift is always (0, 0) for the reference image
    shifts.push(opencv::core::Point::new(0, 0));
    
    // Calculate shifts for remaining images using grayscale
    for i in 1..gray_images.len() {
        let img = gray_images.get(i).unwrap();
        let shift = align_mtb.calculate_shift(&reference, &img)
            .map_err(|e| format!("Failed to calculate shift for image {}: {}", i, e))?;
        shifts.push(shift);
    }

    println!("    [OPENCV] Calculated shifts for {} images", shifts.len());

    // Align color images using the calculated shifts
    let mut aligned_images: Vector<Mat> = Vector::new();
    
    // Reference image stays as is (color)
    aligned_images.push(color_images.get(0).unwrap().clone());
    
    // Align remaining color images
    for i in 1..gray_images.len() {
        let color_img = color_images.get(i).unwrap();
        let shift = shifts.get(i).unwrap().clone();
        let mut aligned: Mat = Mat::default();
        align_mtb.shift_mat(&color_img, &mut aligned, shift)
            .map_err(|e| format!("Failed to shift image {}: {}", i, e))?;
        aligned_images.push(aligned);
    }

    // Save aligned images
    let mut aligned_files = Vec::new();

    // Save all aligned images (including reference)
    // Use no compression to preserve 16-bit depth
    let tiff_params: Vector<i32> = vec![
        IMWRITE_TIFF_COMPRESSION as i32,
        1,  // No compression (preserves 16-bit)
    ].into_iter().collect();
    
    for (idx, aligned_img) in aligned_images.iter().enumerate() {
        let out_filename = format!("opencv_set_{}_{:04}.tif", set_idx, idx + 1);
        let out_path = align_folder.join(&out_filename);
        imwrite(&out_path.to_string_lossy(), &aligned_img, &tiff_params)
            .map_err(|e| format!("Failed to save {}: {}", out_path.display(), e))?;
        aligned_files.push(out_path);
    }

    // Create log entry
    let log_file = logs_dir.join(format!("opencv_align_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== OpenCV Alignment (AlignMTB) - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Input files: {}\n", source_files.len()));
    for file in source_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str(&format!("\nOutput files: {}\n", aligned_files.len()));
    for file in &aligned_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str(&format!("\nShifts:\n"));
    for (i, shift) in shifts.iter().enumerate() {
        log_content.push_str(&format!("  Image {}: dx={}, dy={}\n", i + 1, shift.x, shift.y));
    }
    log_content.push_str(&format!("\nProcessing time: {:.2}s\n", set_start.elapsed().as_secs_f32()));
    log_content.push_str("\n✓ Alignment completed using OpenCV AlignMTB algorithm.\n");

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    println!("    [OPENCV] Set {}: ✓ Complete (Time: {:.2}s)",
        set_idx, set_start.elapsed().as_secs_f32());

    Ok(aligned_files)
}

/// Alternative alignment using KeyPoint matching
///
/// This method uses feature detection (ORB/SIFT) and matching.
#[allow(dead_code)]
pub fn align_set_with_keypoints(
    source_files: &[PathBuf],
    align_folder: &Path,
    set_idx: usize,
    logs_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let set_start = Instant::now();

    println!("    [OPENCV] Aligning {} files with KeyPoint matching...", source_files.len());

    // Create align output directory
    if let Err(e) = std::fs::create_dir_all(align_folder) {
        return Err(format!("Failed to create align directory: {}", e));
    }

    // Load all images (grayscale for processing)
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
        return Err("No images to align".to_string());
    }

    // For now, just copy the images as a placeholder
    // Full implementation would use ORB/SIFT feature matching
    let mut aligned_files = Vec::new();
    let empty_params: Vector<i32> = Vector::new();
    for (idx, _img) in images.iter().enumerate() {
        let out_filename = format!("opencv_kp_set_{}_{:04}.tif", set_idx, idx + 1);
        let out_path = align_folder.join(&out_filename);
        
        // Save original image (placeholder - full impl would warp based on homography)
        imwrite(&out_path.to_string_lossy(), &images.get(idx).unwrap(), &empty_params)
            .map_err(|e| format!("Failed to save {}: {}", out_path.display(), e))?;
        aligned_files.push(out_path);
    }

    // Create log entry
    let log_file = logs_dir.join(format!("opencv_keypoint_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== OpenCV KeyPoint Alignment (PLACEHOLDER) - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Input files: {}\n", source_files.len()));
    log_content.push_str(&format!("\nOutput files: {}\n", aligned_files.len()));
    log_content.push_str(&format!("\nProcessing time: {:.2}s\n", set_start.elapsed().as_secs_f32()));
    log_content.push_str("\n⚠️  NOTE: KeyPoint alignment is a placeholder.\n");

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    println!("    [OPENCV] Set {}: ✓ Complete (Time: {:.2}s) [PLACEHOLDER]",
        set_idx, set_start.elapsed().as_secs_f32());

    Ok(aligned_files)
}
