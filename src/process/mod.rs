//! HDR Processing module
//!
//! This module handles the HDR creation workflow including:
//! - Aligning bracketed images (align_image_stack or OpenCV AlignMTB)
//! - Merging to HDR (Blender, OpenCV MergeDebevec, or native Rust merger)
//! - Tone mapping (Luminance CLI or OpenCV operators)
//! - RAW processing (RawTherapee CLI)
//! - Applying PP3 profiles
//! - Cleaning up temporary files

mod ev_calc;
mod external_blender;
mod external_luminance;
mod opencv_align;
mod opencv_merge;
mod opencv_tonemap;
mod processor;
pub mod rust_merge;

pub use processor::process_folder;
