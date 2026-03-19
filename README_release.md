# HDR Merge Master v1.0.0

A desktop application for merging bracketed HDR images.

## Requirements

### Required Software

1. **Blender** (for HDR merging)
   - Download from: https://www.blender.org/download/
   - Configure the path in the application Setup dialog

2. **OpenCV Runtime** (opencv_world4130.dll)
   - Included in this release package
   - Must be in the same directory as the executable

### Optional Software

3. **align_image_stack** (from Hugin) - for image alignment
   - Download from: https://hugin.sourceforge.io/download/
   - Configure the path in the application Setup dialog

4. **Luminance HDR CLI** - for tone mapping
   - Download from: https://sourceforge.net/projects/qtpfsgui/
   - Configure the path in the application Setup dialog

5. **RawTherapee CLI** - for RAW file processing
   - Download from: https://rawtherapee.com/
   - Configure the path in the application Setup dialog

## Installation

1. Extract all files to a folder of your choice
2. Run `hdr-merge-master.exe`
3. On first run, the Setup dialog will open
4. Configure the paths to required executables (Blender at minimum)
5. Click "Save" to save your configuration

## Usage

1. **Add Folders**: Click "Add Folder" to select folders containing bracketed HDR sequences
2. **Select Profile**: Choose a processing profile for each folder
3. **Configure Settings**: Use the Setup dialog to configure:
   - Alignment method (align_image_stack or OpenCV AlignMTB)
   - HDR merge method (Blender, OpenCV Debevec, OpenCV Robertson, or Rust)
   - Tone mapping method (Luminance CLI or OpenCV)
   - Number of processing threads
   - And more...
4. **Execute**: Click "Execute" to process all folders

## Files Included

- `hdr-merge-master.exe` - Main application executable
- `opencv_world4130.dll` - OpenCV runtime library (REQUIRED)
- `blender/` - Blender HDR merge scripts
  - `HDR_Merge.blend` - Blender project file
  - `blender_merge.py` - Python script for HDR merging

## Configuration

Configuration is stored in `config.json` in the application data directory:
- **Windows**: `%APPDATA%\hdr-merge-master\config.json`

## Command Line Usage

```bash
hdr-merge-master.exe [OPTIONS]

Options:
  -t, --threads <THREADS>    Number of processing threads
  -a, --align                Enable image alignment
  -v, --verbose              Print detailed progress information
  --use-opencv-align         Use OpenCV AlignMTB instead of align_image_stack
  --use-opencv-debevec       Use OpenCV MergeDebevec for HDR merging
  --use-opencv-tonemap       Use OpenCV tone mapping
  --tonemap-operator <OP>    Tone mapping operator (Reinhard, Drago, Durand, Mantiuk)
  --batch <FILE>             Process folders from batch JSON file
  -h, --help                 Print help
  -V, --version              Print version
```

## Batch Processing

Create a JSON file with the following format:

```json
{
  "folders": [
    {
      "path": "C:/path/to/folder1",
      "profile": "Default",
      "align": true
    },
    {
      "path": "C:/path/to/folder2",
      "profile": "Landscape",
      "align": false
    }
  ]
}
```

Then run:
```bash
hdr-merge-master.exe --batch batch.json
```

## Troubleshooting

### Application won't start
- Ensure `opencv_world4130.dll` is in the same directory as the executable
- Check that Blender is properly configured in the Setup dialog

### HDR merging fails
- Verify Blender path is correctly configured
- Ensure the `blender/` directory is in the same location as the executable
- Check that both `HDR_Merge.blend` and `blender_merge.py` are present

### Alignment fails
- Install align_image_stack from Hugin or enable OpenCV AlignMTB
- Configure the path in the Setup dialog

## License

GPLv3-or-later

## Authors

Savva Zakahrov

## Support

For issues and feature requests, please visit the project repository.
