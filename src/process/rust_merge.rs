//! Native Rust HDR Merger
//!
//! This module implements a custom HDR merging algorithm in pure Rust.
//! It works by:
//! 1. Loading images as f32 linear RGB
//! 2. Normalizing exposure based on EV differences
//! 3. Merging pairs from brightest to darkest
//! 4. Using overexposure-based weighting to capture highlights from darker images
//!
//! The algorithm works on two images at a time, progressively merging
//! darker images to capture highlight details that are overexposed in brighter images.

use image::GenericImageView;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use rayon::prelude::*;

use crate::config::FolderEntry;
use crate::process::ev_calc::calculate_relative_evs;
use crate::scan_folder::ScannedFile;

/// Scaling factor applied to the final merged HDR result
/// This brings the merged values into a standard EXR range
/// Higher values = darker output, Lower values = brighter output
const HDR_OUTPUT_SCALE_FACTOR: f32 = 30.0;

/// Thread-safe log collector for gathering log messages during parallel processing
#[derive(Clone, Debug)]
pub struct LogCollector {
    messages: Arc<Mutex<Vec<String>>>,
}

impl LogCollector {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log(&self, message: String) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.push(message);
        }
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents an HDR image in linear f32 RGB space
#[derive(Clone)]
pub struct LinearImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 3]>, // Linear RGB values
}

#[allow(dead_code)]
impl LinearImage {
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![[0.0; 3]; pixel_count],
        }
    }

    /// Load from a DynamicImage and convert to linear RGB
    /// 
    /// IMPORTANT: This function converts sRGB-encoded images to linear RGB space
    /// by applying the inverse sRGB transfer function.
    pub fn from_image(img: &image::DynamicImage) -> Self {
        let (width, height) = img.dimensions();
        let rgb_img = img.to_rgb32f();

        // Convert from sRGB to linear RGB
        let pixels = rgb_img.pixels()
            .map(|p| [
                srgb_to_linear(p[0]),
                srgb_to_linear(p[1]),
                srgb_to_linear(p[2]),
            ])
            .collect();

        Self {
            width,
            height,
            pixels,
        }
    }

    /// Load image directly to linear RGB format using buffered reader (OPTIMIZED)
    /// Avoids double conversion by reading and converting in one pass
    /// 
    /// IMPORTANT: This function converts sRGB-encoded images (like TIFFs from RawTherapee)
    /// to linear RGB space by applying the inverse sRGB transfer function.
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let load_start = Instant::now();

        // Open file with buffering for better IO performance
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
        let buf_reader = BufReader::with_capacity(1024 * 1024, file); // 1MB buffer

        // Load image
        let img = image::load(buf_reader, image::ImageFormat::from_path(path)
            .map_err(|e| format!("Unknown image format for {}: {}", path.display(), e))?)
            .map_err(|e| format!("Failed to decode {}: {}", path.display(), e))?;

        let (width, height) = img.dimensions();

        // Convert to RGB32F in one pass
        let rgb32f = img.to_rgb32f();

        // Extract pixels and convert from sRGB to linear RGB
        // TIFF files from RawTherapee are typically saved as sRGB with gamma encoding
        let pixels: Vec<[f32; 3]> = rgb32f.pixels()
            .map(|p| [
                srgb_to_linear(p[0]),
                srgb_to_linear(p[1]),
                srgb_to_linear(p[2]),
            ])
            .collect();

        let load_time = load_start.elapsed();
        println!("  [IO] Loaded {} ({}x{}): {:.2?}",
                 path.file_name().unwrap_or_default().to_string_lossy(),
                 width, height, load_time);

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Scale all pixel values by a factor
    /// 
    /// # Arguments
    /// * `factor` - Scaling factor (e.g., 1.0/30.0 to reduce brightness)
    pub fn scale(&mut self, factor: f32) {
        for pixel in &mut self.pixels {
            pixel[0] *= factor;
            pixel[1] *= factor;
            pixel[2] *= factor;
        }
    }

    /// Save as EXR format with linear RGB data (OPTIMIZED - batch write)
    pub fn save_as_exr(&self, path: &Path) -> Result<(), String> {
        let save_start = Instant::now();
        
        // Prepare all pixels upfront for batch writing
        let width = self.width as usize;
        let height = self.height as usize;
        
        let prepare_start = Instant::now();
        let pixels: Vec<(f32, f32, f32)> = self.pixels.iter()
            .map(|p| (p[0], p[1], p[2]))
            .collect();
        let prepare_time = prepare_start.elapsed();
        
        // Write EXR file using batch operation
        let write_start = Instant::now();
        exr::prelude::write_rgb_file(
            &path.to_string_lossy().to_string(),
            width,
            height,
            |x, y| {
                let idx = y * width + x;
                pixels[idx]
            }
        ).map_err(|e| format!("Failed to write EXR file {}: {}", path.display(), e))?;
        let write_time = write_start.elapsed();
        
        let total_time = save_start.elapsed();
        
        println!("  [EXR SAVE] {} ({}x{}):", 
                 path.file_name().unwrap_or_default().to_string_lossy(), width, height);
        println!("    - Prepare pixel buffer: {:.2?}", prepare_time);
        println!("    - Write EXR file: {:.2?}", write_time);
        println!("    - Total save time: {:.2?}", total_time);
        
        Ok(())
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> [f32; 3] {
        let idx = (y * self.width + x) as usize;
        self.pixels[idx]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: [f32; 3]) {
        let idx = (y * self.width + x) as usize;
        self.pixels[idx] = pixel;
    }
}

/// Calculate exposure adjustment factor from EV difference
///
/// # Arguments
/// * `ev_diff` - EV difference (positive = image is darker, needs brightening)
///
/// # Returns
/// Multiplication factor to apply to pixel values
#[inline]
fn ev_to_factor(ev_diff: f32) -> f32 {
    2.0_f32.powf(ev_diff)
}

/// Convert sRGB-encoded value to linear RGB
///
/// TIFF files from RawTherapee and most other sources are saved with sRGB gamma encoding.
/// This function applies the inverse sRGB transfer function to convert to linear light values.
///
/// # Arguments
/// * `srgb` - sRGB-encoded value (0.0 to 1.0)
///
/// # Returns
/// Linear RGB value (0.0 to 1.0)
#[inline]
fn srgb_to_linear(srgb: f32) -> f32 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB value to sRGB-encoded
///
/// This is used when saving images that need to be viewed on sRGB displays.
/// EXR files should store linear data, so this is NOT used for EXR output.
///
/// # Arguments
/// * `linear` - Linear RGB value (0.0 to 1.0)
///
/// # Returns
/// sRGB-encoded value (0.0 to 1.0)
#[inline]
#[allow(dead_code)]
fn linear_to_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Calculate overexposure weight for a pixel
///
/// Returns a weight from 0.0 (completely overexposed) to 1.0 (not overexposed)
/// Uses a simplified algorithm with gamma adjustment and linear mapping
///
/// Algorithm:
/// 1. Find maximum of RGB channels
/// 2. Apply gamma adjustment (0.4545)
/// 3. Calculate dot product with [0.6, 0.6, 0.6]
/// 4. Multiply by 0.57735
/// 5. Map result from 0.5-1.0 range to 0.0-1.0 range
///
/// # Arguments
/// * `pixel` - Linear RGB pixel values
/// * `_threshold` - Threshold where overexposure starts (unused in new algorithm)
///
/// # Returns
/// Weight from 0.0 to 1.0
#[inline]
fn overexposure_weight(pixel: [f32; 3], _threshold: f32) -> f32 {
    // Step 1: Find maximum channel value
    let max_channel = pixel[0].max(pixel[1]).max(pixel[2]);
    
    // Step 2: Apply gamma adjustment (0.4545 = 1/2.2)
    let gamma = 0.4545f32;
    let gamma_adjusted = max_channel.powf(gamma);
    
    // Step 3: Calculate dot product with [0.6, 0.6, 0.6] for uniform RGB
    // let dot1 = gamma_adjusted * 0.6 + gamma_adjusted * 0.6 + gamma_adjusted * 0.6;
    
    // Step 4: Multiply by 0.57735 (approximately 1/sqrt(3))
    // let scaled1 = dot1 * 0.57735;
    
    // Step 5: Map result from 0.75-1.0 range to 0.0-1.0 range
    let weight = 1.0 - ((gamma_adjusted - _threshold) / _threshold).clamp(0.0, 1.0);
    
    weight
}

/// Calculate luminance-based weight for a pixel
///
/// Returns higher weights for mid-tones, lower for very dark or very bright
///
/// # Arguments
/// * `pixel` - Linear RGB pixel values
///
/// # Returns
/// Weight from 0.0 to 1.0
#[inline]
fn luminance_weight(pixel: [f32; 3]) -> f32 {
    // Rec. 709 luminance coefficients
    let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];

    // Peak at mid-tones (around 0.18)
    let mid = 0.18;
    let dist = (lum - mid).abs();
    (1.0 - dist.clamp(0.0, 1.0)).powi(2)
}

/// Save a weight mask as a grayscale PNG image
///
/// # Arguments
/// * `weights` - Slice of weight values (0.0 to 1.0)
/// * `width` - Image width
/// * `height` - Image height
/// * `path` - Output file path
///
/// # Returns
/// Result indicating success or error
fn save_weight_mask(
    weights: &[f32],
    width: u32,
    height: u32,
    path: &Path,
) -> Result<(), String> {
    let save_start = Instant::now();

    // Convert weights to grayscale image (0-255)
    let img_data: Vec<u8> = weights
        .iter()
        .map(|w| (w.clamp(0.0, 1.0) * 255.0) as u8)
        .collect();

    // Create grayscale image
    let img = image::GrayImage::from_raw(width, height, img_data)
        .ok_or_else(|| format!("Failed to create weight mask image ({}x{})", width, height))?;

    // Save as PNG
    img.save(path)
        .map_err(|e| format!("Failed to save weight mask {}: {}", path.display(), e))?;

    let save_time = save_start.elapsed();
    println!("  [WEIGHT MASK] Saved {} ({:.2?})", path.display(), save_time);

    Ok(())
}

/// Merge two HDR images using overexposure-based weighting
///
/// The algorithm:
/// 1. Adjust both images to a common exposure baseline
/// 2. For each pixel, calculate weights based on overexposure (using ORIGINAL images before adjustment)
/// 3. Darker image contributes more where brighter image is overexposed
/// 4. Blend using calculated weights
///
/// # Arguments
/// * `bright_img` - Brighter image (shorter exposure, captures highlights)
/// * `dark_img` - Darker image (longer exposure, captures shadows)
/// * `bright_ev` - EV value of bright image (relative, brightest = 0)
/// * `dark_ev` - EV value of dark image (relative, positive value)
/// * `debug_export` - Optional folder path to export debug EXR files
/// * `debug_prefix` - Prefix for debug output filenames
/// * `log` - Optional log collector for gathering messages
///
/// # Returns
/// Merged HDR image
pub fn merge_pair(
    bright_img: &LinearImage,
    dark_img: &LinearImage,
    bright_ev: f32,
    dark_ev: f32,
    debug_export: Option<&Path>,
    debug_prefix: &str,
    log: Option<&LogCollector>,
) -> LinearImage {
    assert_eq!(bright_img.width, dark_img.width);
    assert_eq!(bright_img.height, dark_img.height);

    let merge_start = Instant::now();
    let width = bright_img.width;
    let height = bright_img.height;

    // Calculate exposure adjustment to normalize both images
    // We'll normalize to the brightest image's exposure (EV = 0)
    let ev_calc_start = Instant::now();
    let dark_to_bright_factor = ev_to_factor(dark_ev - bright_ev);
    let ev_calc_time = ev_calc_start.elapsed();
    let msg = format!("  [MERGE] EV calculation: {:.2?} (factor: {:.3})", ev_calc_time, dark_to_bright_factor);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    // Export adjusted dark image if debug is enabled
    if let Some(debug_dir) = debug_export {
        let adjusted_dark = LinearImage {
            width,
            height,
            pixels: dark_img.pixels.iter()
                .map(|p| [
                    p[0] * dark_to_bright_factor,
                    p[1] * dark_to_bright_factor,
                    p[2] * dark_to_bright_factor,
                ])
                .collect(),
        };
        let debug_path = debug_dir.join(format!("{}_adjusted_dark.exr", debug_prefix));
        if let Err(e) = adjusted_dark.save_as_exr(&debug_path) {
            eprintln!("  [DEBUG] Failed to save adjusted dark image: {}", e);
        } else {
            println!("  [DEBUG] Exported: {}", debug_path.display());
        }
    }

    // Process pixels in parallel, collecting results and weights
    let pixel_process_start = Instant::now();
    
    // Collect pixels and weights together in parallel, then separate
    let pixel_weight_data: Vec<([f32; 3], f32, f32, f32)> = (0..bright_img.pixels.len()).into_par_iter().map(|i| {
        let bright_pixel = bright_img.pixels[i];
        let dark_pixel_original = dark_img.pixels[i];

        // Adjust dark image to match bright image's exposure
        let dark_pixel_adjusted = [
            dark_pixel_original[0] * dark_to_bright_factor,
            dark_pixel_original[1] * dark_to_bright_factor,
            dark_pixel_original[2] * dark_to_bright_factor,
        ];

        // Calculate overexposure weights using ORIGINAL images (before exposure adjustment)
        // This ensures weights are based on actual captured data, not adjusted values
        // Bright image weight: high where it's NOT overexposed (good highlight data)
        let bright_weight = overexposure_weight(bright_pixel, 0.5);

        // Dark image weight: high where bright image IS overexposed
        // (dark image has better data there)
        let dark_weight = 1.0 - bright_weight;

        // Add luminance weighting for smoother transitions (using original pixels)
        let bright_lum_weight = luminance_weight(bright_pixel);
        let dark_lum_weight = luminance_weight(dark_pixel_original);

        // Combine weights
        let total_weight = bright_weight * bright_lum_weight + dark_weight * dark_lum_weight;
        // let total_weight = dark_weight + bright_weight;

        let merged_pixel = if total_weight > 0.0001 {
            // Weighted blend using ADJUSTED dark pixel
            let bright_contrib = bright_weight * bright_lum_weight;
            let dark_contrib = dark_weight * dark_lum_weight;

            [
                (bright_pixel[0] * bright_contrib + dark_pixel_adjusted[0] * dark_contrib) / total_weight,
                (bright_pixel[1] * bright_contrib + dark_pixel_adjusted[1] * dark_contrib) / total_weight,
                (bright_pixel[2] * bright_contrib + dark_pixel_adjusted[2] * dark_contrib) / total_weight,
            ]
        } else {
            // Fallback to simple average
            [
                (bright_pixel[0] + dark_pixel_adjusted[0]) * 0.5,
                (bright_pixel[1] + dark_pixel_adjusted[1]) * 0.5,
                (bright_pixel[2] + dark_pixel_adjusted[2]) * 0.5,
            ]
        };

        // Return pixel and weights (weights only used for debug export)
        (merged_pixel, bright_weight, dark_weight, total_weight)
    }).collect();
    
    // Separate pixels and weights
    let pixels: Vec<[f32; 3]> = pixel_weight_data.iter().map(|(p, _, _, _)| *p).collect();
    let bright_weights: Vec<f32> = pixel_weight_data.iter().map(|(_, bw, _, _)| *bw).collect();
    let dark_weights: Vec<f32> = pixel_weight_data.iter().map(|(_, _, dw, _)| *dw).collect();
    let combined_weights: Vec<f32> = pixel_weight_data.iter().map(|(_, _, _, cw)| *cw).collect();
    
    let pixel_process_time = pixel_process_start.elapsed();

    // Export weight masks if debug is enabled
    if let Some(debug_dir) = debug_export {
        // Save bright overexposure weight mask
        let bright_mask_path = debug_dir.join(format!("{}_bright_weight.png", debug_prefix));
        if let Err(e) = save_weight_mask(&bright_weights, width, height, &bright_mask_path) {
            eprintln!("  [DEBUG] Failed to save bright weight mask: {}", e);
        }

        // Save dark overexposure weight mask
        let dark_mask_path = debug_dir.join(format!("{}_dark_weight.png", debug_prefix));
        if let Err(e) = save_weight_mask(&dark_weights, width, height, &dark_mask_path) {
            eprintln!("  [DEBUG] Failed to save dark weight mask: {}", e);
        }

        // Save combined weight mask
        let combined_mask_path = debug_dir.join(format!("{}_combined_weight.png", debug_prefix));
        if let Err(e) = save_weight_mask(&combined_weights, width, height, &combined_mask_path) {
            eprintln!("  [DEBUG] Failed to save combined weight mask: {}", e);
        }
    }

    let total_time = merge_start.elapsed();
    let msg = format!("  [MERGE] Pixel processing ({} MPixels): {:.2?}",
             (width * height) as f32 / 1_000_000.0, pixel_process_time);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }
    
    let msg = format!("  [MERGE] Total merge time: {:.2?}", total_time);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    let merged = LinearImage {
        width,
        height,
        pixels,
    };

    // Export merged result if debug is enabled
    if let Some(debug_dir) = debug_export {
        let debug_path = debug_dir.join(format!("{}_merged.exr", debug_prefix));
        if let Err(e) = merged.save_as_exr(&debug_path) {
            let msg = format!("  [DEBUG] Failed to save merged image: {}", e);
            eprintln!("{}", msg);
            if let Some(l) = log { l.log(msg); }
        } else {
            let msg = format!("  [DEBUG] Exported: {}", debug_path.display());
            println!("{}", msg);
            if let Some(l) = log { l.log(msg); }
        }
    }

    merged
}

/// Merge multiple bracketed images into an HDR
///
/// Images should be sorted from brightest (shortest exposure) to darkest (longest exposure).
/// The algorithm progressively merges pairs, starting with the brightest two images,
/// then merging the result with the next darker image, and so on.
///
/// # Arguments
/// * `linear_images` - Slice of pre-loaded LinearImage (already in f32 RGB format)
/// * `ev_values` - Relative EV values (brightest image should be 0.0, darker = positive)
/// * `debug_export` - Optional folder path to export debug EXR files
/// * `set_idx` - Bracket set index for naming debug files
/// * `log` - Optional log collector for gathering messages
///
/// # Returns
/// Merged HDR image, or None if input is empty
pub fn merge_bracket_sequence(
    linear_images: &[LinearImage],
    ev_values: &[f32],
    debug_export: Option<&Path>,
    set_idx: usize,
    log: Option<&LogCollector>,
) -> Option<LinearImage> {
    if linear_images.is_empty() || linear_images.len() != ev_values.len() {
        return None;
    }

    if linear_images.len() == 1 {
        return Some(linear_images[0].clone());
    }

    let seq_start = Instant::now();
    let msg = format!("[BRACKET_SEQ] Starting merge of {} images", linear_images.len());
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    // Create debug subfolder for this set if debug is enabled
    let debug_dir;
    if let Some(base_debug_dir) = debug_export {
        debug_dir = base_debug_dir.join(format!("set_{:03}", set_idx));
        std::fs::create_dir_all(&debug_dir).ok();

        // Export source images
        for (i, img) in linear_images.iter().enumerate() {
            let debug_path = debug_dir.join(format!("source_{:03}_ev_{:.2}.exr", i, ev_values[i]));
            if let Err(e) = img.save_as_exr(&debug_path) {
                let msg = format!("  [DEBUG] Failed to save source image {}: {}", i, e);
                eprintln!("{}", msg);
                if let Some(l) = log { l.log(msg); }
            } else {
                let msg = format!("  [DEBUG] Exported source: {}", debug_path.display());
                println!("{}", msg);
                if let Some(l) = log { l.log(msg); }
            }
        }
    } else {
        debug_dir = PathBuf::new();
    }

    // Start with the brightest image (index 0)
    let mut current = linear_images[0].clone();
    let mut current_ev = ev_values[0];

    // Progressively merge with darker images
    for i in 1..linear_images.len() {
        let pair_start = Instant::now();
        let next = &linear_images[i];
        let next_ev = ev_values[i];

        let debug_prefix = format!("step_{:02}", i);
        let debug_path = if debug_export.is_some() { Some(debug_dir.as_path()) } else { None };

        let msg = format!("[BRACKET_SEQ] Merging pair {} (EV: {:.2} + {:.2} -> {:.2})",
                 i, current_ev, next_ev, (current_ev + next_ev) * 0.5);
        println!("{}", msg);
        if let Some(l) = log { l.log(msg); }

        // Merge current result with next darker image
        let merged = merge_pair(&current, next, current_ev, next_ev, debug_path, &debug_prefix, log);

        // Update current to merged result
        // The merged image has an effective EV closer to the brighter image
        // but with extended highlight range from the darker image
        current = merged;
        current_ev = current_ev * 0.5; // Blend EV towards middle

        let msg = format!("[BRACKET_SEQ] Pair {} completed in {:.2?}", i, pair_start.elapsed());
        println!("{}", msg);
        if let Some(l) = log { l.log(msg); }
    }

    let msg = format!("[BRACKET_SEQ] Total sequence merge time: {:.2?}", seq_start.elapsed());
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    // Apply output scaling factor to bring HDR values into standard EXR range
    let mut result = current;
    result.scale(1.0 / HDR_OUTPUT_SCALE_FACTOR);
    let msg = format!("[BRACKET_SEQ] Applied output scale factor: 1/{}", HDR_OUTPUT_SCALE_FACTOR);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    Some(result)
}

/// Merge bracketed images from file paths (OPTIMIZED)
///
/// Uses direct loading to LinearImage format (no double conversion)
/// and buffered IO for faster file reading.
///
/// # Arguments
/// * `file_paths` - Paths to bracketed images (sorted brightest to darkest)
/// * `ev_values` - Relative EV values
/// * `debug_export` - Optional folder path to export debug EXR files
/// * `set_idx` - Bracket set index for naming debug files
/// * `log` - Optional log collector for gathering messages
///
/// # Returns
/// Merged HDR image, or error message
pub fn merge_from_files(
    file_paths: &[String],
    ev_values: &[f32],
    debug_export: Option<&Path>,
    set_idx: usize,
    log: Option<&LogCollector>,
) -> Result<LinearImage, String> {
    if file_paths.is_empty() {
        return Err("No input files provided".to_string());
    }

    if file_paths.len() != ev_values.len() {
        return Err(format!(
            "File count ({}) doesn't match EV count ({})",
            file_paths.len(),
            ev_values.len()
        ));
    }

    let total_start = Instant::now();
    let msg = format!("[MERGE_FROM_FILES] Starting merge of {} files", file_paths.len());
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    // Log file names and EV values
    for (i, (path, ev)) in file_paths.iter().zip(ev_values.iter()).enumerate() {
        let path_name = Path::new(path).file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let msg = format!("  [INPUT] File {}: {} (EV: {:.2})", i, path_name, ev);
        println!("{}", msg);
        if let Some(l) = log { l.log(msg); }
    }

    // Load all images directly to LinearImage format (single conversion)
    let load_start = Instant::now();
    let linear_images: Result<Vec<LinearImage>, _> = file_paths
        .iter()
        .map(|path| LinearImage::load_from_path(Path::new(path)))
        .collect();
    let load_time = load_start.elapsed();

    let linear_images = linear_images?;
    let msg = format!("[MERGE_FROM_FILES] Total image loading: {:.2?}", load_time);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    // Verify all images have the same dimensions
    let dim_check_start = Instant::now();
    let (first_width, first_height) = (linear_images[0].width, linear_images[0].height);
    for (i, img) in linear_images.iter().enumerate().skip(1) {
        if img.width != first_width || img.height != first_height {
            return Err(format!(
                "Image {} has different dimensions ({}x{}) than first image ({}x{})",
                i, img.width, img.height, first_width, first_height
            ));
        }
    }
    let msg = format!("[MERGE_FROM_FILES] Dimension check: {:.2?}", dim_check_start.elapsed());
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    let merge_start = Instant::now();
    let result = merge_bracket_sequence(&linear_images, ev_values, debug_export, set_idx, log)
        .ok_or("Failed to merge bracket sequence".to_string());
    let merge_time = merge_start.elapsed();

    let total_time = total_start.elapsed();
    let msg = format!("[MERGE_FROM_FILES] Merge processing: {:.2?}", merge_time);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }
    
    let msg = format!("[MERGE_FROM_FILES] TOTAL TIME (IO + Processing): {:.2?}", total_time);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }
    
    let msg = format!("[MERGE_FROM_FILES]   - IO (load): {:.2?} ({:.1}%)", load_time, (load_time.as_secs_f32() / total_time.as_secs_f32()) * 100.0);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }
    
    let msg = format!("[MERGE_FROM_FILES]   - Processing: {:.2?} ({:.1}%)", merge_time.as_secs_f32(), (merge_time.as_secs_f32() / total_time.as_secs_f32()) * 100.0);
    println!("{}", msg);
    if let Some(l) = log { l.log(msg); }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ev_to_factor() {
        // EV 0 = factor 1.0 (no change)
        assert!((ev_to_factor(0.0) - 1.0).abs() < 0.0001);
        // EV 1 = factor 2.0 (double brightness)
        assert!((ev_to_factor(1.0) - 2.0).abs() < 0.0001);
        // EV -1 = factor 0.5 (half brightness)
        assert!((ev_to_factor(-1.0) - 0.5).abs() < 0.0001);
        // EV 3 = factor 8.0
        assert!((ev_to_factor(3.0) - 8.0).abs() < 0.0001);
    }
    
    #[test]
    fn test_overexposure_weight() {
        // Test the new gamma-based overexposure weight algorithm
        // Iterate through the full 0.0-1.0 range in 0.1 increments
        println!("\n=== Overexposure Weight Test (Gamma-based Algorithm) ===");
        println!("Input (RGB)          -> Weight");
        println!("----------------------------------------");
        
        for i in 0..=10 {
            let value = (i as f32) * 0.1;
            let pixel = [value, value, value];
            let weight = overexposure_weight(pixel, 0.8);
            
            // Verify weight is in valid range
            assert!(weight >= 0.0 && weight <= 1.0, 
                "Weight {} out of range for input {}", weight, value);
            
            println!("[{:.1}, {:.1}, {:.1}]  -> {:.6}", value, value, value, weight);
        }
        
        println!("----------------------------------------");
        
        // Test specific cases
        println!("\nSpecific test cases:");
        
        // Very dark pixels
        let dark_weight = overexposure_weight([0.0, 0.0, 0.0], 0.8);
        println!("Black [0.0, 0.0, 0.0] -> {:.6}", dark_weight);
        assert!(dark_weight >= 0.0 && dark_weight <= 1.0);
        
        // Mid-tone pixels
        let mid_weight = overexposure_weight([0.5, 0.5, 0.5], 0.8);
        println!("Mid   [0.5, 0.5, 0.5] -> {:.6}", mid_weight);
        assert!(mid_weight >= 0.0 && mid_weight <= 1.0);
        
        // Bright pixels
        let bright_weight = overexposure_weight([0.8, 0.8, 0.8], 0.8);
        println!("Bright[0.8, 0.8, 0.8] -> {:.6}", bright_weight);
        assert!(bright_weight >= 0.0 && bright_weight <= 1.0);
        
        // Very bright pixels
        let very_bright_weight = overexposure_weight([0.9, 0.9, 0.9], 0.8);
        println!("V.Bright [0.9, 0.9, 0.9] -> {:.6}", very_bright_weight);
        assert!(very_bright_weight >= 0.0 && very_bright_weight <= 1.0);
        
        // White pixels
        let white_weight = overexposure_weight([1.0, 1.0, 1.0], 0.8);
        println!("White [1.0, 1.0, 1.0] -> {:.6}", white_weight);
        assert!(white_weight >= 0.0 && white_weight <= 1.0);
        
        // Test with non-uniform RGB values
        println!("\nNon-uniform RGB values:");
        let mixed1 = overexposure_weight([1.0, 0.5, 0.2], 0.8);
        println!("[1.0, 0.5, 0.2] -> {:.6}", mixed1);
        
        let mixed2 = overexposure_weight([0.3, 0.9, 0.6], 0.8);
        println!("[0.3, 0.9, 0.6] -> {:.6}", mixed2);
        
        let mixed3 = overexposure_weight([0.1, 0.1, 0.95], 0.8);
        println!("[0.1, 0.1, 0.95] -> {:.6}", mixed3);
    }
    
    #[test]
    fn test_linear_image_creation() {
        let img = LinearImage::new(100, 100);
        assert_eq!(img.width, 100);
        assert_eq!(img.height, 100);
        assert_eq!(img.pixels.len(), 10000);
    }
}

/// Merge bracketed images using Rust native merger (concurrent version)
///
/// This function processes multiple bracket sets concurrently using a thread pool.
/// Each bracket set is merged independently using the native Rust HDR merger.
///
/// # Arguments
/// * `files` - Flattened list of all aligned image paths (sorted by bracket set)
/// * `exr_folder` - Output folder for merged EXR files
/// * `ev_source_files` - Original scanned files for EV calculation
/// * `folder` - Folder entry with processing metadata
/// * `logs_dir` - Directory for log files
/// * `total_sets` - Number of bracket sets to process
/// * `threads` - Number of concurrent threads to use
/// * `debug_export` - Optional base folder path to export debug EXR files
///
/// # Returns
/// Result indicating success or error message
pub fn merge_with_rust_concurrent(
    files: &[PathBuf],
    exr_folder: &Path,
    ev_source_files: &[ScannedFile],
    folder: &FolderEntry,
    logs_dir: &Path,
    total_sets: u32,
    threads: usize,
    debug_export: Option<&Path>,
) -> Result<(), String> {
    use std::fs;

    let concurrent_start = Instant::now();
    let msg = format!("[RUST_MERGE_CONCURRENT] Starting concurrent merge ({} sets, {} threads)", total_sets, threads);
    println!("{}", msg);
    
    if debug_export.is_some() {
        println!("[RUST_MERGE_CONCURRENT] Debug export enabled");
    }

    // Create output directory
    let dir_start = Instant::now();
    fs::create_dir_all(exr_folder)
        .map_err(|e| format!("Failed to create EXR folder: {}", e))?;
    println!("[RUST_MERGE_CONCURRENT] Created output folder: {:.2?}", dir_start.elapsed());

    // Process bracket sets in parallel, collecting log messages and timing
    let process_start = Instant::now();
    
    // Create log collectors for each set
    let log_collectors: Vec<LogCollector> = (0..total_sets as usize)
        .map(|_| LogCollector::new())
        .collect();
    
    let results: Vec<Result<(usize, Result<String, String>), String>> = (0..total_sets as usize)
        .into_par_iter()
        .map(|set_idx| {
            let set_start = Instant::now();
            let log = log_collectors[set_idx].clone();
            
            let msg = format!("[SET {}] Starting processing", set_idx);
            println!("{}", msg);
            log.log(msg);

            let start_idx = set_idx * folder.brackets as usize;
            let end_idx = start_idx + folder.brackets as usize;

            if end_idx > files.len() {
                return Err(format!(
                    "Invalid bracket range for set {}: {}-{}",
                    set_idx, start_idx, end_idx
                ));
            }

            let bracket_paths = &files[start_idx..end_idx];
            let bracket_ev_files = &ev_source_files[start_idx..end_idx];

            // Log input files with their exposure info
            log.log(format!("[SET {}] Input files:", set_idx));
            for (i, (path, ev_file)) in bracket_paths.iter().zip(bracket_ev_files.iter()).enumerate() {
                let file_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                let ev_info = ev_file.exposure_time.as_ref()
                    .map(|e| format!("exp={}", e))
                    .unwrap_or_else(|| "exp=unknown".to_string());
                log.log(format!("  [INPUT] File {}: {} ({})", i, file_name, ev_info));
            }

            // Calculate EV values for this bracket set
            let ev_start = Instant::now();
            let ev_values = calculate_relative_evs(bracket_ev_files);
            let msg = format!("[SET {}] EV calculation: {:.2?}", set_idx, ev_start.elapsed());
            println!("{}", msg);
            log.log(msg);

            // Log EV values for each file
            for (i, (path, ev)) in bracket_paths.iter().zip(ev_values.iter()).enumerate() {
                let file_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                log.log(format!("  [EV] File {}: {} -> EV: {:.2}", i, file_name, ev));
            }

            // Sort images by EV (brightest first = lowest EV)
            let sort_start = Instant::now();
            let mut indexed_images: Vec<_> = bracket_paths.iter().zip(ev_values.iter()).collect();
            indexed_images.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
            let msg = format!("[SET {}] Sorting by EV: {:.2?}", set_idx, sort_start.elapsed());
            println!("{msg}");
            log.log(msg);

            // Load images in sorted order
            let sorted_paths: Vec<_> = indexed_images.iter().map(|(path, _)| (*path).clone()).collect();
            let sorted_evs: Vec<_> = indexed_images.iter().map(|(_, ev)| **ev).collect();

            // Log sorted order
            log.log(format!("[SET {}] Sorted processing order (by EV, brightest first):", set_idx));
            for (i, (path, ev)) in sorted_paths.iter().zip(sorted_evs.iter()).enumerate() {
                let file_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                log.log(format!("  [ORDER] {}: {} (EV: {:.2})", i, file_name, ev));
            }

            // Merge using Rust native merger (has its own detailed timing)
            let merge_start = Instant::now();
            let merged = merge_from_files(
                &sorted_paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
                &sorted_evs,
                debug_export,
                set_idx,
                Some(&log),
            )?;
            let msg = format!("[SET {}] Merge completed: {:.2?}", set_idx, merge_start.elapsed());
            println!("{}", msg);
            log.log(msg);

            // Generate output filename
            let output_name = format!("merged_{:03}.exr", set_idx);

            // Save as EXR format with linear RGB data (has its own detailed timing)
            let save_start = Instant::now();
            let exr_output = exr_folder.join(&output_name);
            merged.save_as_exr(&exr_output)?;
            let msg = format!("[SET {}] Save completed: {:.2?}", set_idx, save_start.elapsed());
            println!("{}", msg);
            log.log(msg);

            let set_time = set_start.elapsed();
            let msg = format!("[SET {}] TOTAL set processing time: {:.2?}", set_idx, set_time);
            println!("{}", msg);
            log.log(msg);

            // Return log message
            Ok((set_idx, Ok(format!("Set {}: Merged {} images -> {} ({:.2?})",
                set_idx, bracket_paths.len(), output_name, set_time))))
        })
        .collect();

    let process_time = process_start.elapsed();
    let msg = format!("[RUST_MERGE_CONCURRENT] Parallel processing completed: {:.2?}", process_time);
    println!("{}", msg);

    // Collect and check results
    let results: Vec<(usize, Result<String, String>)> = results.into_iter()
        .filter_map(|r| r.ok())
        .collect();
    
    let log_messages: Vec<String> = results.iter()
        .filter_map(|(_, result)| result.as_ref().ok())
        .cloned()
        .collect();

    // Create log file and write all messages
    let log_start = Instant::now();
    let log_path = logs_dir.join(format!(
        "rust_merge_{}.log",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    
    // Write header
    let mut log_content = Vec::new();
    log_content.push("====== Rust HDR Merge Log ======".to_string());
    log_content.push(format!("Started: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    log_content.push(format!("Input sets: {}, Threads: {}", total_sets, threads));
    log_content.push(format!("Debug export: {}", if debug_export.is_some() { "enabled" } else { "disabled" }));
    log_content.push(format!("Parallel processing time: {:.2?}", process_time));
    log_content.push(String::new()); // Empty line
    
    // Add per-set logs
    for (set_idx, _) in results.iter() {
        log_content.push(format!("====== SET {} ======", set_idx));
        let set_logs = log_collectors[*set_idx].get_messages();
        log_content.extend(set_logs);
        log_content.push(String::new()); // Empty line
    }
    
    // Add summary
    log_content.push("====== SUMMARY ======".to_string());
    for msg in &log_messages {
        log_content.push(msg.clone());
    }
    let total_time = concurrent_start.elapsed();
    log_content.push(format!("Total concurrent merge time: {:.2?}", total_time));
    log_content.push(format!("Average per set: {:.2?}", total_time / total_sets));
    log_content.push(format!("Throughput: {:.1} sets/minute", (total_sets as f32) / (total_time.as_secs_f32() / 60.0)));
    log_content.push(format!("Completed: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    
    // Write to file
    let mut log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Failed to create log file: {}", e))?;

    use std::io::Write;
    for line in &log_content {
        writeln!(log_file, "{}", line).map_err(|e| e.to_string())?;
    }
    
    let msg = format!("[RUST_MERGE_CONCURRENT] Log file written: {:.2?}", log_start.elapsed());
    println!("{}", msg);

    let total_time = concurrent_start.elapsed();
    println!("[RUST_MERGE_CONCURRENT] ====== SUMMARY ======");
    println!("[RUST_MERGE_CONCURRENT] Total concurrent merge time: {:.2?}", total_time);
    println!("[RUST_MERGE_CONCURRENT] Average per set: {:.2?}", total_time / total_sets);
    println!("[RUST_MERGE_CONCURRENT] Throughput: {:.1} sets/minute", (total_sets as f32) / (total_time.as_secs_f32() / 60.0));

    Ok(())
}
