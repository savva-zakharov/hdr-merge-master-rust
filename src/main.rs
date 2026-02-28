#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod process;
mod scan_folder;
mod ui;

use eframe::egui;
use ui::HdrMergeApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "HDR Merge Master 1.0.0",
        options,
        Box::new(|cc| Ok(Box::new(HdrMergeApp::new(cc)))),
    )
}
