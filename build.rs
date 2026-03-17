//! Build script for Windows resource compilation

fn main() {
    // Only compile Windows resources on Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        
        // Set the icon path
        res.set_icon("icons/icon.ico");
        
        // Set version information
        res.set("FileVersion", "1.0.0.0");
        res.set("ProductVersion", "1.0.0.0");
        res.set("CompanyName", "HDR Merge Master");
        res.set("FileDescription", "HDR Merge Master - Desktop Application");
        res.set("LegalCopyright", "Copyright (C) 2026");
        res.set("OriginalFilename", "hdr-merge-master.exe");
        res.set("ProductName", "HDR Merge Master");
        
        // Compile the resources
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", e);
        }
    }
}
