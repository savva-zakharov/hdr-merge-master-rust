//! EV (Exposure Value) calculation module
//!
//! This module calculates relative EV differences between images
//! based on their EXIF data (shutter speed, aperture, ISO).
//!
//! EV difference is calculated as:
//! EV = log2(shutter_ratio) + log2(aperture_ratio) + log2(ISO_ratio)
//!
//! Where:
//! - shutter_ratio = bright_shutter / dark_shutter
//! - aperture_ratio = dark_aperture / bright_aperture
//! - ISO_ratio = bright_ISO / dark_ISO

use std::f32::consts::LN_2;

/// Parse exposure time string to f32 (in seconds)
///
/// # Arguments
/// * `exp_str` - Exposure time string (e.g., "1/100", "0.5", "2")
///
/// # Returns
/// Exposure time in seconds, or None if parsing fails
pub fn parse_exposure_time(exp_str: &str) -> Option<f32> {
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

/// Parse f-number string to f32
///
/// # Arguments
/// * `fnum_str` - F-number string (e.g., "f/8", "8.0", "5.6")
///
/// # Returns
/// F-number as f32, or None if parsing fails
pub fn parse_f_number(fnum_str: &str) -> Option<f32> {
    let fnum_str = fnum_str.trim();

    // Handle "f/8" format
    if fnum_str.starts_with('f') || fnum_str.starts_with('F') {
        if let Some(pos) = fnum_str.find('/') {
            if let Ok(val) = fnum_str[pos + 1..].trim().parse::<f32>() {
                return Some(val);
            }
        }
    }

    // Handle plain number format
    fnum_str.parse::<f32>().ok()
}

/// Parse ISO string to f32
///
/// # Arguments
/// * `iso_str` - ISO string (e.g., "100", "400", "ISO100")
///
/// # Returns
/// ISO value as f32, or None if parsing fails
pub fn parse_iso(iso_str: &str) -> Option<f32> {
    let iso_str = iso_str.trim();

    // Handle "ISO100" format
    let iso_str = if iso_str.starts_with("ISO") || iso_str.starts_with("iso") {
        &iso_str[3..]
    } else {
        iso_str
    };

    iso_str.parse::<f32>().ok()
}

/// Calculate relative EV values for a bracket sequence
///
/// Returns EV values relative to the BRIGHTEST image (which gets EV = 0).
/// Images with LESS exposure (darker) get POSITIVE EV values.
/// This matches the Blender Python script's expected input format.
///
/// # Arguments
/// * `files` - Slice of ScannedFile with EXIF data
///
/// # Returns
/// Vector of relative EV values (brightest image = 0.0, darker = positive)
pub fn calculate_relative_evs(
    files: &[crate::scan_folder::ScannedFile],
) -> Vec<f32> {
    if files.is_empty() {
        return Vec::new();
    }

    // Find the brightest image (longest exposure time) to use as reference
    let brightest_idx = files.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let a_shutter = a.exposure_time.as_ref()
                .and_then(|s| parse_exposure_time(s))
                .unwrap_or(0.01);
            let b_shutter = b.exposure_time.as_ref()
                .and_then(|s| parse_exposure_time(s))
                .unwrap_or(0.01);
            a_shutter.partial_cmp(&b_shutter).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    let mut evs = Vec::with_capacity(files.len());

    // Get reference (brightest) image exposure parameters
    let ref_shutter = files[brightest_idx]
        .exposure_time
        .as_ref()
        .and_then(|s| parse_exposure_time(s))
        .unwrap_or(0.01);

    let ref_aperture = files[brightest_idx]
        .f_number
        .as_ref()
        .and_then(|s| parse_f_number(s))
        .unwrap_or(8.0);

    let ref_iso = files[brightest_idx]
        .iso
        .as_ref()
        .and_then(|s| parse_iso(s))
        .unwrap_or(100.0);

    // Calculate EV difference for each image relative to the brightest
    for file in files {
        let curr_shutter = file
            .exposure_time
            .as_ref()
            .and_then(|s| parse_exposure_time(s))
            .unwrap_or(0.01);

        let curr_aperture = file
            .f_number
            .as_ref()
            .and_then(|s| parse_f_number(s))
            .unwrap_or(8.0);

        let curr_iso = file
            .iso
            .as_ref()
            .and_then(|s| parse_iso(s))
            .unwrap_or(100.0);

        // Calculate EV using Python formula: EV = log2(bright/dark) for shutter and ISO
        // For aperture: EV = 2 * log2(dark/bright) because smaller f-number = more light
        // Here ref=brightest, curr=current image
        // EV = log2(ref_shutter/curr_shutter) + 2*log2(curr_aperture/ref_aperture) + log2(ref_iso/curr_iso)
        let dr_shutter = if ref_shutter > 0.0 && curr_shutter > 0.0 {
            (ref_shutter / curr_shutter).ln() / LN_2
        } else {
            0.0
        };

        let dr_aperture = if ref_aperture > 0.0 && curr_aperture > 0.0 {
            (curr_aperture / ref_aperture).ln() / LN_2 * 2.0
        } else {
            0.0
        };

        let dr_iso = if ref_iso > 0.0 && curr_iso > 0.0 {
            (ref_iso / curr_iso).ln() / LN_2
        } else {
            0.0
        };

        let ev = dr_shutter + dr_aperture + dr_iso;
        evs.push(ev);
    }

    evs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exposure_time_fraction() {
        assert!((parse_exposure_time("1/100").unwrap() - 0.01).abs() < 0.0001);
        assert!((parse_exposure_time("1/50").unwrap() - 0.02).abs() < 0.0001);
    }

    #[test]
    fn test_parse_exposure_time_decimal() {
        assert!((parse_exposure_time("0.5").unwrap() - 0.5).abs() < 0.0001);
        assert!((parse_exposure_time("2").unwrap() - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_parse_f_number() {
        assert!((parse_f_number("f/8").unwrap() - 8.0).abs() < 0.01);
        assert!((parse_f_number("5.6").unwrap() - 5.6).abs() < 0.01);
    }

    #[test]
    fn test_parse_iso() {
        assert!((parse_iso("100").unwrap() - 100.0).abs() < 0.01);
        assert!((parse_iso("ISO400").unwrap() - 400.0).abs() < 0.01);
    }
}
