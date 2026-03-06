#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod config;
mod process;
mod scan_folder;
mod ui;

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = cli::Cli::parse_args();

    // If CLI mode is enabled, run in headless mode
    if cli.is_cli_mode() {
        return run_cli_mode(cli);
    }

    // Otherwise, run the GUI
    run_gui_mode()
}

/// Run in headless CLI mode
fn run_cli_mode(cli: cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    use cli::BatchFile;
    use std::fs;

    println!("HDR Merge Master v{}", env!("CARGO_PKG_VERSION"));
    println!("Running in CLI mode\n");

    // Load configuration
    let config_path = config::get_config_path()?;
    let mut config = if config_path.exists() {
        config::Config::load(&config_path)
            .unwrap_or_else(|_| config::Config::default())
    } else {
        config::Config::default()
    };

    // Update config with CLI options
    config.gui_settings.threads = cli.threads as u8;
    config.gui_settings.do_cleanup = cli.cleanup;
    config.gui_settings.do_align = cli.align;
    config.gui_settings.use_opencv_align = cli.use_opencv_align;
    config.gui_settings.use_opencv_merge = cli.use_opencv_merge;
    config.gui_settings.use_opencv_tonemap = cli.use_opencv_tonemap;
    config.gui_settings.tonemap_operator = cli.tonemap_operator;
    config.gui_settings.tonemap_intensity = cli.tonemap_intensity;
    config.gui_settings.tonemap_contrast = cli.tonemap_contrast;
    config.gui_settings.tonemap_saturation = cli.tonemap_saturation;

    // Collect folders to process
    let mut folders_to_process: Vec<(PathBuf, Option<String>)> = Vec::new();

    // Handle --batch option
    if let Some(batch_path) = &cli.batch {
        println!("Loading batch file: {}", batch_path.display());
        let batch_content = fs::read_to_string(batch_path)
            .map_err(|e| format!("Failed to read batch file: {}", e))?;
        let batch_file: BatchFile = serde_json::from_str(&batch_content)
            .map_err(|e| format!("Failed to parse batch file: {}", e))?;

        for entry in &batch_file.folders {
            folders_to_process.push((PathBuf::from(&entry.path), entry.profile.clone()));
        }
        println!("Loaded {} folders from batch file", batch_file.folders.len());
    }

    // Handle --folder option
    if let Some(folder_path) = &cli.folder {
        if cli.recursive {
            println!("Scanning folder recursively: {}", folder_path.display());
            // Scan folder recursively for subfolders
            scan_folder_recursive(folder_path, &mut folders_to_process, cli.profile.clone())?;
        } else {
            folders_to_process.push((folder_path.clone(), cli.profile.clone()));
        }
    }

    // Handle process subcommand
    if let Some(cli::Commands::Process { folders }) = &cli.command {
        for folder in folders {
            folders_to_process.push((folder.clone(), cli.profile.clone()));
        }
    }

    if folders_to_process.is_empty() {
        eprintln!("Error: No folders specified. Use --folder, --batch, or process subcommand.");
        std::process::exit(1);
    }

    println!("Processing {} folders with {} threads\n", folders_to_process.len(), config.gui_settings.threads);

    // Process each folder
    let mut success_count = 0;
    let mut error_count = 0;

    for (folder_path, profile_override) in folders_to_process {
        println!("\n{}", "=".repeat(60));
        println!("Processing: {}", folder_path.display());
        if let Some(ref profile) = profile_override {
            println!("Profile: {}", profile);
        }
        println!("{}", "=".repeat(60));

        // Scan the folder
        let scan_result = scan_folder::scan_folder(
            &folder_path,
            &config.gui_settings.processed_extensions,
            &config.gui_settings.raw_extensions
        );

        if scan_result.files.is_empty() {
            eprintln!("Warning: No valid bracket sets found in {}", folder_path.display());
            error_count += 1;
            continue;
        }

        // Create folder entry
        let profile_name = profile_override
            .or_else(|| cli.profile.clone())
            .unwrap_or_else(|| config.gui_settings.pp3_file.clone());

        let folder_entry = config::FolderEntry {
            path: folder_path.to_string_lossy().to_string(),
            profile: profile_name,
            extension: String::new(),
            is_raw: scan_result.is_raw,
            align: cli.align,
            brackets: scan_result.brackets,
            sets: scan_result.files.len() as u32 / scan_result.brackets,
            files: scan_result.files,
        };

        // Process the folder
        let gui_settings: config::GuiSettings = (&config.gui_settings).into();
        match process::process_folder(&folder_entry, &config, &gui_settings) {
            Ok(_) => {
                success_count += 1;
                println!("✓ Successfully processed: {}", folder_path.display());
            }
            Err(e) => {
                error_count += 1;
                eprintln!("✗ Error processing {}: {}", folder_path.display(), e);
            }
        }
    }

    // Print summary
    println!("\n{}", "=".repeat(60));
    println!("Processing complete!");
    println!("  Successful: {}", success_count);
    println!("  Failed: {}", error_count);
    println!("{}", "=".repeat(60));

    if error_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Scan folder recursively for subfolders
fn scan_folder_recursive(
    folder: &std::path::Path,
    folders: &mut Vec<(std::path::PathBuf, Option<String>)>,
    profile: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !folder.is_dir() {
        return Ok(());
    }

    // Check if this folder contains image files
    let contains_images = std::fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .any(|e| {
            let path = e.path();
            if !path.is_file() {
                return false;
            }
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "tif" | "tiff" | "jpg" | "jpeg" | "png" | "dng" | "cr2" | "cr3" | "nef" | "arw" | "raf" | "orf" | "rw2" | "pef"
                    )
                })
                .unwrap_or(false)
        });

    if contains_images {
        folders.push((folder.to_path_buf(), profile.clone()));
    }

    // Recurse into subdirectories
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_folder_recursive(&path, folders, profile.clone())?;
        }
    }

    Ok(())
}

/// Run in GUI mode
fn run_gui_mode() -> Result<(), Box<dyn std::error::Error>> {
    iced::application(ui::HdrMergeApp::new, ui::HdrMergeApp::update, ui::HdrMergeApp::view)
        .subscription(ui::HdrMergeApp::subscription)
        .theme(ui::HdrMergeApp::theme)
        .window_size((1000.0, 600.0))
        .run()?;

    Ok(())
}
