//! OpenCV-based tone mapping for HDR to LDR conversion
//!
//! This module provides multiple tone mapping operators for converting
//! HDR images to displayable LDR images.
//!
//! Uses the image crate for loading EXR files (via exr crate),
//! and OpenCV for tone mapping operators.

use std::path::{Path, PathBuf};
use std::time::Instant;

use opencv::{
    prelude::*,
    photo::{
        create_tonemap_reinhard, create_tonemap_drago,
        create_tonemap_mantiuk,
    },
    imgcodecs::{imwrite, IMWRITE_JPEG_QUALITY},
    core::{Vector, Mat},
};

/// Tone mapping operator selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMappingOperator {
    Reinhard,
    Drago,
    Mantiuk,
}

impl Default for ToneMappingOperator {
    fn default() -> Self {
        ToneMappingOperator::Reinhard
    }
}

impl std::fmt::Display for ToneMappingOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToneMappingOperator::Reinhard => write!(f, "Reinhard"),
            ToneMappingOperator::Drago => write!(f, "Drago"),
            ToneMappingOperator::Mantiuk => write!(f, "Mantiuk"),
        }
    }
}

/// Parameters for tone mapping
#[derive(Debug, Clone)]
pub struct ToneMappingParams {
    pub operator: ToneMappingOperator,
    pub intensity: f32,      // Overall intensity (0-1)
    pub contrast: f32,       // Contrast enhancement
    pub saturation: f32,     // Color saturation
    pub detail: f32,         // Detail enhancement (for some operators)
}

impl Default for ToneMappingParams {
    fn default() -> Self {
        Self {
            operator: ToneMappingOperator::Reinhard,
            intensity: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            detail: 0.0,
        }
    }
}

/// Apply tone mapping to an HDR image
///
/// # Arguments
/// * `hdr_image` - Input HDR image (32-bit float)
/// * `params` - Tone mapping parameters
///
/// # Returns
/// Tone mapped LDR image (8-bit)
pub fn apply_tone_mapping(
    hdr_image: &Mat,
    params: &ToneMappingParams,
) -> Result<Mat, String> {
    let mut tonemap_result = Mat::default();

    // Ensure the input image is in the correct format (CV_32FC3)
    // OpenCV tone mapping requires 32-bit float, 3 channels
    let mut hdr_normalized = Mat::default();
    if hdr_image.depth() != opencv::core::CV_32F {
        // Convert to 32-bit float if needed
        hdr_image.convert_to(&mut hdr_normalized, opencv::core::CV_32FC3, 1.0 / 65535.0, 0.0)
            .map_err(|e| format!("Failed to convert image to 32-bit float: {}", e))?;
    } else {
        hdr_normalized = hdr_image.clone();
    }

    // Create and apply tone mapping operator
    match params.operator {
        ToneMappingOperator::Reinhard => {
            let mut tonemap = create_tonemap_reinhard(params.intensity, params.contrast, params.saturation, params.detail)
                .map_err(|e| format!("Failed to create Reinhard tonemap: {}", e))?;
            tonemap.process(&hdr_normalized, &mut tonemap_result)
                .map_err(|e| format!("Reinhard tone mapping failed: {}", e))?;
        }
        ToneMappingOperator::Drago => {
            let mut tonemap = create_tonemap_drago(params.intensity, params.contrast, params.saturation)
                .map_err(|e| format!("Failed to create Drago tonemap: {}", e))?;
            tonemap.process(&hdr_normalized, &mut tonemap_result)
                .map_err(|e| format!("Drago tone mapping failed: {}", e))?;
        }
        ToneMappingOperator::Mantiuk => {
            let mut tonemap = create_tonemap_mantiuk(params.contrast, params.saturation, params.intensity)
                .map_err(|e| format!("Failed to create Mantiuk tonemap: {}", e))?;
            tonemap.process(&hdr_normalized, &mut tonemap_result)
                .map_err(|e| format!("Mantiuk tone mapping failed: {}", e))?;
        }
    }

    // Convert from 32-bit float to 8-bit for saving
    let mut ldr_result_8u = Mat::default();
    tonemap_result.convert_to(&mut ldr_result_8u, opencv::core::CV_8UC3, 255.0, 0.0)
        .map_err(|e| format!("Failed to convert to 8-bit: {}", e))?;

    Ok(ldr_result_8u)
}

/// Tone map HDR files to JPG using OpenCV
///
/// # Arguments
/// * `hdr_files` - List of HDR file paths (TIFF or EXR)
/// * `jpg_folder` - Output directory for JPG files
/// * `params` - Tone mapping parameters
/// * `logs_dir` - Directory to save log files
/// * `threads` - Number of concurrent threads
///
/// # Returns
/// Result indicating success
pub fn tone_map_hdr_to_jpg_opencv(
    hdr_files: &[PathBuf],
    jpg_folder: &Path,
    params: &ToneMappingParams,
    logs_dir: &Path,
    threads: usize,
) -> Result<(), String> {
    if hdr_files.is_empty() {
        return Err("No HDR files to tone map".to_string());
    }

    // Create output directory
    if let Err(e) = std::fs::create_dir_all(jpg_folder) {
        return Err(format!("Failed to create output directory: {}", e));
    }

    // Process files in parallel
    use rayon::prelude::*;
    let results: Vec<Result<(), String>> = hdr_files
        .par_iter()
        .with_max_len(threads)
        .map(|hdr_path| {
            let file_start = Instant::now();
            let filename = hdr_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            println!("[OPENCV-TONEMAP] Processing: {}", filename);

            // Load HDR image using image crate (supports EXR via exr crate)
            let hdr_img = image::open(&hdr_path)
                .map_err(|e| format!("Failed to load {}: {}", hdr_path.display(), e))?;

            // Convert to 32-bit float RGB for OpenCV tone mapping
            let rgb_img = hdr_img.to_rgb32f();
            let (width, height) = rgb_img.dimensions();
            
            // Create OpenCV Mat from image data
            let pixels = rgb_img.as_raw();
            
            // Create 3-channel 32-bit float Mat with correct size
            let mut hdr_mat = Mat::new_rows_cols_with_default(height as i32, width as i32, opencv::core::CV_32FC3, opencv::core::Scalar::default())
                .map_err(|e| format!("Failed to create Mat: {}", e))?;
            
            // Copy pixel data (image crate uses RGB, OpenCV expects BGR)
            // Get raw data pointer and copy manually
            let mat_data = hdr_mat.data_mut();
            if mat_data.is_null() {
                return Err("Failed to get Mat data pointer".to_string());
            }
            
            for (i, pixel) in pixels.chunks(3).enumerate() {
                let idx = i * 3 * 4; // 3 channels * 4 bytes per f32
                unsafe {
                    // Convert RGB to BGR for OpenCV and write as f32
                    let b_ptr = mat_data.add(idx) as *mut f32;
                    let g_ptr = mat_data.add(idx + 4) as *mut f32;
                    let r_ptr = mat_data.add(idx + 8) as *mut f32;
                    *b_ptr = pixel[2];
                    *g_ptr = pixel[1];
                    *r_ptr = pixel[0];
                }
            }

            // Apply tone mapping
            let ldr_image = apply_tone_mapping(&hdr_mat, params)?;

            // Generate output JPG path
            let jpg_path = jpg_folder.join(format!("{}.jpg", hdr_path.file_stem().unwrap().to_string_lossy()));

            // Save as JPG
            let jpg_params: Vector<i32> = vec![
                IMWRITE_JPEG_QUALITY as i32,
                95,  // High quality JPEG
            ].into_iter().collect();

            imwrite(&jpg_path.to_string_lossy(), &ldr_image, &jpg_params)
                .map_err(|e| format!("Failed to save {}: {}", jpg_path.display(), e))?;

            println!("  [OPENCV-TONEMAP] {} ✓ Complete (Time: {:.2}s)", 
                filename, file_start.elapsed().as_secs_f32());

            // Create log entry
            let log_file = logs_dir.join(format!("opencv_tonemap_{}.log", hdr_path.file_stem().unwrap().to_string_lossy()));
            let mut log_content = String::new();
            log_content.push_str(&format!("=== OpenCV Tone Mapping ===\n\n"));
            log_content.push_str(&format!("Input: {}\n", hdr_path.display()));
            log_content.push_str(&format!("Output: {}\n", jpg_path.display()));
            log_content.push_str(&format!("Operator: {}\n", params.operator));
            log_content.push_str(&format!("Intensity: {}\n", params.intensity));
            log_content.push_str(&format!("Contrast: {}\n", params.contrast));
            log_content.push_str(&format!("Saturation: {}\n", params.saturation));
            log_content.push_str(&format!("Processing time: {:.2}s\n", file_start.elapsed().as_secs_f32()));

            if let Err(e) = std::fs::write(&log_file, &log_content) {
                eprintln!("Warning: Failed to write log file: {}", e);
            }

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
