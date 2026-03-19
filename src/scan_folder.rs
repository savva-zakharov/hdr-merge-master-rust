//! Folder scanning module for detecting HDR bracket sequences
//!
//! This module scans a folder for image files, reads their EXIF data,
//! and detects repeating bracket patterns based on exposure settings.

use exif::{In, Tag};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;
use std::path::Path;

/// Represents a single file entry with its EXIF information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    pub path: String,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub bias: Option<String>,
}

/// Result of scanning a folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub files: Vec<ScannedFile>,
    pub is_raw: bool,
    pub brackets: u32,
    pub sets: u32,
}

/// Exposure settings used for bracket detection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ExposureSettings {
    exposure_time: Option<String>,
    f_number: Option<String>,
    iso: Option<String>,
}

impl ExposureSettings {}

/// Read EXIF data from a file
fn read_exif_data(path: &Path) -> Option<ScannedFile> {
    let file = fs::File::open(path).ok()?;
    let mut bufreader = BufReader::new(&file);
    let reader = exif::Reader::new();

    match reader.read_from_container(&mut bufreader) {
        Ok(exif) => {
            let exposure_time = exif
                .get_field(Tag::ExposureTime, In::PRIMARY)
                .map(|f| f.display_value().to_string());

            let f_number = exif
                .get_field(Tag::FNumber, In::PRIMARY)
                .map(|f| f.display_value().to_string());

            let iso = exif
                .get_field(Tag::PhotographicSensitivity, In::PRIMARY)
                .map(|f| f.display_value().to_string());

            let bias_raw = exif
                .get_field(Tag::ExposureBiasValue, In::PRIMARY)
                .map(|f| f.display_value().to_string());

            // Add '+' prefix to positive bias values for consistent display
            let bias = bias_raw.map(|b| {
                let trimmed = b.trim();
                // Check if it's a positive number without a sign
                if trimmed.starts_with('-') || trimmed.starts_with('+') {
                    b // Already has a sign
                } else {
                    format!("+{}", b) // Add '+' prefix
                }
            });

            Some(ScannedFile {
                path: path.to_string_lossy().to_string(),
                exposure_time,
                f_number,
                iso,
                bias,
            })
        }
        Err(_) => Some(ScannedFile {
            path: path.to_string_lossy().to_string(),
            exposure_time: None,
            f_number: None,
            iso: None,
            bias: None,
        }),
    }
}

/// Scan a folder for files with matching extensions
fn scan_files(folder_path: &Path, extensions: &[String]) -> Vec<ScannedFile> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    let ext_with_dot = format!(".{}", ext_lower);

                    if extensions.iter().any(|e| e.to_lowercase() == ext_with_dot) {
                        if let Some(file) = read_exif_data(&path) {
                            files.push(file);
                        }
                    }
                }
            }
        }
    }

    // Sort files by path for consistent ordering
    files.sort_by(|a, b| a.path.cmp(&b.path));

    files
}

/// Detect the bracket pattern from exposure settings
/// Returns the number of unique exposure settings (bracket count)
fn detect_brackets(files: &[ScannedFile]) -> u32 {
    if files.is_empty() {
        return 1;
    }

    // Build a list of exposure settings for each file
    let settings_sequence: Vec<ExposureSettings> = files
        .iter()
        .filter_map(|f| {
            if let (Some(exp), Some(fnum), Some(iso)) = (&f.exposure_time, &f.f_number, &f.iso) {
                Some(ExposureSettings {
                    exposure_time: Some(exp.clone()),
                    f_number: Some(fnum.clone()),
                    iso: Some(iso.clone()),
                })
            } else {
                None
            }
        })
        .collect();

    if settings_sequence.is_empty() {
        // No EXIF data available, assume 1 bracket
        return 1;
    }

    // Try to find the smallest repeating pattern
    // Start from 1 and go up to half the sequence length
    let len = settings_sequence.len();

    for bracket_size in 1..=len {
        if len % bracket_size != 0 {
            continue;
        }

        // Check if this bracket_size creates a valid repeating pattern
        let mut is_valid = true;
        let pattern: Vec<ExposureSettings> = settings_sequence[..bracket_size].to_vec();

        for (i, settings) in settings_sequence.iter().enumerate() {
            if settings != &pattern[i % bracket_size] {
                is_valid = false;
                break;
            }
        }

        if is_valid {
            return bracket_size as u32;
        }
    }

    // If no pattern found, return the total count as a fallback
    len as u32
}

/// Main scanning function
///
/// Scans a folder for image files (both processed and raw), reads EXIF data,
/// and detects bracket patterns.
///
/// # Arguments
/// * `folder_path` - Path to the folder to scan
/// * `processed_extensions` - List of processed file extensions (e.g., [".tif", ".tiff"])
/// * `raw_extensions` - List of raw file extensions (e.g., [".dng", ".cr2"])
///
/// # Returns
/// A `ScanResult` containing the files, is_raw flag, bracket count, and set count
pub fn scan_folder(
    folder_path: &Path,
    processed_extensions: &[String],
    raw_extensions: &[String],
) -> ScanResult {
    // Scan for processed files
    let mut processed_files = scan_files(folder_path, processed_extensions);

    // Scan for raw files
    let mut raw_files = scan_files(folder_path, raw_extensions);

    // Determine if files are raw based on whether we found raw files
    // (check BEFORE appending, as append() moves the data)
    let is_raw = !raw_files.is_empty();

    // Combine both lists
    let mut files = Vec::new();
    files.append(&mut processed_files);
    files.append(&mut raw_files);

    // Sort files by path for consistent ordering
    files.sort_by(|a, b| a.path.cmp(&b.path));

    // Detect bracket pattern
    let brackets = detect_brackets(&files);

    // Calculate number of sets (total files / brackets, rounded down)
    let sets = if brackets > 0 {
        (files.len() as u32) / brackets
    } else {
        0
    };

    ScanResult {
        files,
        is_raw,
        brackets,
        sets,
    }
}

/// Scan a folder recursively for subfolders containing image files
///
/// # Arguments
/// * `folder_path` - Path to the root folder to scan
/// * `processed_extensions` - List of processed file extensions
/// * `raw_extensions` - List of raw file extensions
/// * `ignore_folders` - List of folder names to ignore
/// * `max_depth` - Maximum depth to recurse (1 = immediate subfolders only)
///
/// # Returns
/// A vector of paths to subfolders that contain image files
pub fn scan_folder_recursive(
    folder_path: &Path,
    processed_extensions: &[String],
    raw_extensions: &[String],
    ignore_folders: &[String],
    max_depth: u32,
) -> Vec<String> {
    let mut result_folders = Vec::new();
    
    scan_folder_recursive_helper(
        folder_path,
        processed_extensions,
        raw_extensions,
        ignore_folders,
        max_depth,
        0,
        &mut result_folders,
    );
    
    result_folders
}

/// Helper function for recursive folder scanning
fn scan_folder_recursive_helper(
    folder_path: &Path,
    processed_extensions: &[String],
    raw_extensions: &[String],
    ignore_folders: &[String],
    max_depth: u32,
    current_depth: u32,
    result_folders: &mut Vec<String>,
) {
    // Stop if we've reached max depth
    if current_depth >= max_depth {
        return;
    }

    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if !path.is_dir() {
                continue;
            }

            // Get folder name for ignore check
            if let Some(folder_name) = path.file_name() {
                let folder_name_str = folder_name.to_string_lossy().to_lowercase();
                
                // Check if this folder should be ignored
                if ignore_folders.iter().any(|ignore| {
                    ignore.to_lowercase() == folder_name_str
                }) {
                    continue;
                }
            }

            // Check if this folder contains image files
            let contains_images = folder_contains_images(&path, processed_extensions, raw_extensions);
            
            if contains_images {
                // Add this folder to results
                result_folders.push(path.to_string_lossy().to_string());
            }

            // Recurse into subfolder
            scan_folder_recursive_helper(
                &path,
                processed_extensions,
                raw_extensions,
                ignore_folders,
                max_depth,
                current_depth + 1,
                result_folders,
            );
        }
    }
}

/// Check if a folder contains image files (without reading EXIF)
fn folder_contains_images(
    folder_path: &Path,
    processed_extensions: &[String],
    raw_extensions: &[String],
) -> bool {
    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    let ext_with_dot = format!(".{}", ext_lower);

                    let is_processed = processed_extensions.iter().any(|e| e.to_lowercase() == ext_with_dot);
                    let is_raw = raw_extensions.iter().any(|e| e.to_lowercase() == ext_with_dot);
                    
                    if is_processed || is_raw {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_detection_pattern() {
        // Create a mock sequence: A, B, C, A, B, C (brackets = 3, sets = 2)
        let files = vec![
            ScannedFile {
                path: "img001.dng".to_string(),
                exposure_time: Some("1/100".to_string()),
                f_number: Some("f/8".to_string()),
                iso: Some("100".to_string()),
                bias: Some("0".to_string()),
            },
            ScannedFile {
                path: "img002.dng".to_string(),
                exposure_time: Some("1/50".to_string()),
                f_number: Some("f/8".to_string()),
                iso: Some("100".to_string()),
                bias: Some("0".to_string()),
            },
            ScannedFile {
                path: "img003.dng".to_string(),
                exposure_time: Some("1/25".to_string()),
                f_number: Some("f/8".to_string()),
                iso: Some("100".to_string()),
                bias: Some("0".to_string()),
            },
            ScannedFile {
                path: "img004.dng".to_string(),
                exposure_time: Some("1/100".to_string()),
                f_number: Some("f/8".to_string()),
                iso: Some("100".to_string()),
                bias: Some("0".to_string()),
            },
            ScannedFile {
                path: "img005.dng".to_string(),
                exposure_time: Some("1/50".to_string()),
                f_number: Some("f/8".to_string()),
                iso: Some("100".to_string()),
                bias: Some("0".to_string()),
            },
            ScannedFile {
                path: "img006.dng".to_string(),
                exposure_time: Some("1/25".to_string()),
                f_number: Some("f/8".to_string()),
                iso: Some("100".to_string()),
                bias: Some("0".to_string()),
            },
        ];

        let brackets = detect_brackets(&files);
        assert_eq!(brackets, 3);
    }
}
