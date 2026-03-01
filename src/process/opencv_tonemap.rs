//! OpenCV-based tone mapping for HDR to LDR conversion
//!
//! This module provides multiple tone mapping operators for converting
//! HDR images to displayable LDR images.

use std::path::{Path, PathBuf};
use std::time::Instant;

use opencv::{
    prelude::*,
    photo::{
        create_tonemap_reinhard, create_tonemap_drago, 
        create_tonemap_mantiuk,
    },
    imgcodecs::{imread, imwrite, IMREAD_COLOR, IMWRITE_JPEG_QUALITY},
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

    // Create and apply tone mapping operator
    match params.operator {
        ToneMappingOperator::Reinhard => {
            let mut tonemap = create_tonemap_reinhard(params.intensity, params.contrast, params.saturation, params.detail)
                .map_err(|e| format!("Failed to create Reinhard tonemap: {}", e))?;
            tonemap.process(hdr_image, &mut tonemap_result)
                .map_err(|e| format!("Reinhard tone mapping failed: {}", e))?;
        }
        ToneMappingOperator::Drago => {
            let mut tonemap = create_tonemap_drago(params.intensity, params.contrast, params.saturation)
                .map_err(|e| format!("Failed to create Drago tonemap: {}", e))?;
            tonemap.process(hdr_image, &mut tonemap_result)
                .map_err(|e| format!("Drago tone mapping failed: {}", e))?;
        }
        ToneMappingOperator::Mantiuk => {
            let mut tonemap = create_tonemap_mantiuk(params.contrast, params.saturation, params.intensity)
                .map_err(|e| format!("Failed to create Mantiuk tonemap: {}", e))?;
            tonemap.process(hdr_image, &mut tonemap_result)
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

            // Load HDR image
            let hdr_image = imread(&hdr_path.to_string_lossy(), IMREAD_COLOR)
                .map_err(|e| format!("Failed to load {}: {}", hdr_path.display(), e))?;
            
            if hdr_image.empty() {
                return Err(format!("Loaded empty image from {}", hdr_path.display()));
            }

            // Apply tone mapping
            let ldr_image = apply_tone_mapping(&hdr_image, params)?;

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
