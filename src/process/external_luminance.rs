//! Luminance HDR CLI tone mapping module
//!
//! Handles tone mapping HDR images to JPG using Luminance HDR CLI

use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Tone map EXR files to JPG with parallel processing using Luminance HDR CLI
pub fn tone_map_exr_to_jpg_concurrent(
    exr_folder: &Path,
    jpg_folder: &Path,
    luminance_exe: &str,
    logs_dir: &Path,
    threads: usize,
) -> Result<(), String> {
    if luminance_exe.is_empty() {
        return Err("Luminance CLI not configured in setup".to_string());
    }

    // Create JPG output directory
    if let Err(e) = std::fs::create_dir_all(jpg_folder) {
        return Err(format!("Failed to create jpg directory: {}", e));
    }

    // Get list of EXR files
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

    if exr_files.is_empty() {
        return Err("No EXR files found for tone mapping".to_string());
    }

    // Sort by filename
    exr_files.sort();

    // Process files in parallel with limited concurrency
    use rayon::prelude::*;
    let results: Vec<Result<(), String>> = exr_files
        .par_iter()
        .with_max_len(threads)
        .map(|exr_path| {
            let file_start = Instant::now();
            let filename = exr_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            println!("[TONEMAP] File {}/{}: {}", exr_files.iter().position(|p| p == exr_path).unwrap_or(0) + 1, exr_files.len(), filename);

            // Generate output JPG path
            let jpg_path = jpg_folder.join(format!("{}.jpg", exr_path.file_stem().unwrap().to_string_lossy()));

            // Build Luminance CLI command
            // luminance-hdr-cli -l exr_path --tmo reinhard02 -q 98 -o jpg_path
            let mut cmd = Command::new(luminance_exe);
            cmd.arg("-l")
                .arg(exr_path)
                .arg("--tmo")
                .arg("reinhard02")
                .arg("-q")
                .arg("98")
                .arg("-o")
                .arg(&jpg_path);

            // Execute command
            let output = cmd.output()
                .map_err(|e| format!("Failed to execute Luminance CLI: {}", e))?;

            // Save logs
            let log_file = logs_dir.join(format!("tonemap_{}.log", exr_path.file_stem().unwrap().to_string_lossy()));
            let mut log_content = String::new();
            log_content.push_str(&format!("=== Luminance Tone Mapping ===\n\n"));
            log_content.push_str(&format!("File: {}\n", exr_path.display()));
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
                println!("  [TONEMAP] File {}: ✗ Failed", filename);
                return Err(format!(
                    "Tone mapping failed for {}: {}\n",
                    exr_path.display(), stderr
                ));
            }

            println!("  [TONEMAP] File {}: ✓ Complete", filename);
            println!("  [TONEMAP] File {}: Time: {:.2}s", filename, file_start.elapsed().as_secs_f32());

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

/// Tone map a single EXR file to JPG using Luminance CLI
pub fn tone_map_single_file(
    exr_path: &Path,
    jpg_path: &Path,
    luminance_exe: &str,
    logs_dir: &Path,
    set_idx: usize,
) -> Result<(), String> {
    use std::fs;

    if luminance_exe.is_empty() {
        return Err("Luminance CLI not configured".to_string());
    }

    // Create JPG output directory
    if let Some(parent) = jpg_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!("Failed to create jpg directory: {}", e));
        }
    }

    // Build Luminance CLI command
    let mut cmd = Command::new(luminance_exe);
    cmd.arg("-l")
        .arg(exr_path)
        .arg("--tmo")
        .arg("reinhard02")
        .arg("-q")
        .arg("98")
        .arg("-o")
        .arg(jpg_path);

    // Execute command
    let output = cmd.output()
        .map_err(|e| format!("Failed to execute Luminance CLI: {}", e))?;

    // Save logs
    let log_file = logs_dir.join(format!("tonemap_set_{:03}.log", set_idx));
    let mut log_content = String::new();
    log_content.push_str(&format!("=== Luminance Tone Mapping - Set {} ===\n\n", set_idx));
    log_content.push_str(&format!("Input: {}\n", exr_path.display()));
    log_content.push_str(&format!("Output: {}\n", jpg_path.display()));
    log_content.push_str(&format!("Command: {:?}\n\n", cmd));
    log_content.push_str("STDOUT:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stdout));
    log_content.push_str("\nSTDERR:\n");
    log_content.push_str(&String::from_utf8_lossy(&output.stderr));
    let _ = fs::write(&log_file, &log_content);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Tone mapping failed: {}\n", stderr));
    }

    println!("    [TONEMAP] Set {}: ✓ Complete", set_idx);

    Ok(())
}
