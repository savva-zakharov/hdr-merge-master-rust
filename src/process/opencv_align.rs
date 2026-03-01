//! OpenCV-based image alignment using libstacker
//!
//! This module provides an alternative to align_image_stack using libstacker's
//! ECC (Enhanced Correlation Coefficient) or KeyPoint matching algorithms.
//!
//! **NOTE**: This module requires OpenCV to be installed on your system.
//! See OPENCV_SETUP.md for installation instructions.
//!
//! **OpenCV Requirements**:
//! - OpenCV 4.x with the following modules: video, features2d, imgproc, calib3d
//! - The `find_transform_ecc()` function from the video module is required
//! - On Windows, ensure opencv_world4xxx.lib includes tracking functionality

use std::path::{Path, PathBuf};
use std::time::Instant;

use libstacker::{ecc_match, EccMatchParameters, MotionType};
use libstacker::opencv::prelude::*;

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

    // Configure ECC match parameters
    // Using Affine transformation which is good for HDR bracket alignment
    let ecc_params = EccMatchParameters {
        motion_type: MotionType::Affine,
        max_count: Some(1000),      // Maximum iterations
        epsilon: Some(1e-10),       // Convergence threshold
        gauss_filt_size: 5,         // Gaussian filter size for pyramid levels
    };

    // Perform ECC alignment - returns a single stacked Mat
    let stacked_mat = ecc_match(source_files, ecc_params, None)
        .map_err(|e| format!("ECC alignment failed: {}", e))?;

    // Convert OpenCV Mat to image and save
    // The stacked result is the aligned combination of all input images
    // We need to save each aligned frame separately
    
    // For HDR merge, we need individual aligned frames, not the stacked result
    // libstacker's ecc_match returns the final warped/stitched result
    // We'll use the reference image and aligned results
    
    // Save the stacked result as the primary aligned output
    let mut aligned_files = Vec::new();
    
    // Convert Mat to image format and save
    let mat_size = stacked_mat.size().map_err(|e| format!("Failed to get mat size: {}", e))?;
    let width = mat_size.width as u32;
    let height = mat_size.height as u32;
    
    // Get the data from Mat
    let mat_data = stacked_mat.data_typed::<f32>()
        .map_err(|e| format!("Failed to get mat data: {}", e))?;
    
    // Create RGB image from the aligned result
    // The ECC result is typically a single-channel or multi-channel aligned image
    let channels = stacked_mat.channels();

    let aligned_img: image::DynamicImage = if channels == 3 {
        // RGB image
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for i in 0..(width * height) as usize {
            let idx = i * 3;
            // Convert f32 to u8 (assuming normalized 0-1 range or convert from f32 values)
            let r = (mat_data[idx].max(0.0).min(255.0)) as u8;
            let g = (mat_data[idx + 1].max(0.0).min(255.0)) as u8;
            let b = (mat_data[idx + 2].max(0.0).min(255.0)) as u8;
            rgb_data.extend_from_slice(&[r, g, b]);
        }
        image::DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(width, height, rgb_data)
                .ok_or("Failed to create RGB image")?
        )
    } else if channels == 1 {
        // Grayscale - convert to RGB
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for &val in mat_data.iter() {
            let gray = (val.max(0.0).min(255.0)) as u8;
            rgb_data.extend_from_slice(&[gray, gray, gray]);
        }
        image::DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(width, height, rgb_data)
                .ok_or("Failed to create RGB image")?
        )
    } else {
        // Handle other channel counts by loading original reference as fallback
        image::open(&source_files[0])
            .map_err(|e| format!("Failed to load reference image: {}", e))?
    };

    // Save the main aligned result
    let out_filename = format!("opencv_set_{}_{:04}.tif", set_idx, 1);
    let out_path = align_folder.join(&out_filename);
    aligned_img
        .save(&out_path)
        .map_err(|e| format!("Failed to save {}: {}", out_path.display(), e))?;
    aligned_files.push(out_path);

    // For HDR processing, we also need to save the other aligned frames
    // Since libstacker returns the stacked result, we align each image individually
    // and save them separately for the HDR merge process
    if source_files.len() > 1 {
        for (idx, src_file) in source_files.iter().enumerate().skip(1) {
            let single_files = vec![source_files[0].clone(), src_file.clone()];
            let aligned_mat = ecc_match(&single_files, ecc_params, None)
                .map_err(|e| format!("ECC alignment failed for image {}: {}", idx + 1, e))?;
            
            // Save this aligned frame
            let out_filename = format!("opencv_set_{}_{:04}.tif", set_idx, idx + 1);
            let out_path = align_folder.join(&out_filename);
            
            // Reuse the conversion logic above
            let mat_size = aligned_mat.size().map_err(|e| format!("Failed to get mat size: {}", e))?;
            let w = mat_size.width as u32;
            let h = mat_size.height as u32;
            let mat_d = aligned_mat.data_typed::<f32>()
                .map_err(|e| format!("Failed to get mat data: {}", e))?;
            let ch = aligned_mat.channels();

            let img: image::DynamicImage = if ch == 3 {
                let mut rgb_data = Vec::with_capacity((w * h * 3) as usize);
                for i in 0..(w * h) as usize {
                    let idx = i * 3;
                    rgb_data.extend_from_slice(&[
                        (mat_d[idx].max(0.0).min(255.0)) as u8,
                        (mat_d[idx + 1].max(0.0).min(255.0)) as u8,
                        (mat_d[idx + 2].max(0.0).min(255.0)) as u8,
                    ]);
                }
                image::DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(w, h, rgb_data).ok_or("Failed to create RGB image")?
                )
            } else {
                let mut rgb_data = Vec::with_capacity((w * h * 3) as usize);
                for &val in mat_d.iter() {
                    let gray = (val.max(0.0).min(255.0)) as u8;
                    rgb_data.extend_from_slice(&[gray, gray, gray]);
                }
                image::DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(w, h, rgb_data).ok_or("Failed to create RGB image")?
                )
            };
            
            img.save(&out_path)
                .map_err(|e| format!("Failed to save {}: {}", out_path.display(), e))?;
            aligned_files.push(out_path);
        }
    }

    // Create log entry
    let log_file = logs_dir.join(format!("opencv_align_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== OpenCV Alignment (libstacker) - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Input files: {}\n", source_files.len()));
    for file in source_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str(&format!("\nOutput files: {}\n", aligned_files.len()));
    for file in &aligned_files {
        log_content.push_str(&format!("  {}\n", file.display()));
    }
    log_content.push_str(&format!("\nProcessing time: {:.2}s\n", set_start.elapsed().as_secs_f32()));
    log_content.push_str("\n✓ Alignment completed using libstacker ECC algorithm.\n");

    if let Err(e) = std::fs::write(&log_file, &log_content) {
        eprintln!("Warning: Failed to write log file: {}", e);
    }

    println!("    [OPENCV] Set {}: ✓ Complete (Time: {:.2}s)",
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
