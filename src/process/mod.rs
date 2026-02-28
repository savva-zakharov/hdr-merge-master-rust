//! HDR Processing module
//!
//! This module handles the HDR creation workflow including:
//! - Aligning bracketed images (align_image_stack or OpenCV AlignMTB)
//! - Merging to HDR
//! - Applying PP3 profiles
//! - Cleaning up temporary files

mod ev_calc;
mod opencv_align;
mod processor;

pub use processor::process_folder;
