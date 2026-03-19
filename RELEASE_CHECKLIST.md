# HDR Merge Master - Release Checklist

## Pre-Release Preparation

### 1. Build Verification
- [ ] Run `cargo build --release` successfully
- [ ] Run `cargo test` - all tests pass
- [ ] Run `cargo clippy` - no critical warnings
- [ ] Verify app icon is embedded (check .exe properties on Windows)

### 2. Binary Dependencies
- [ ] `opencv_world4130.dll` is available
  - Location: Usually in OpenCV installation directory
  - Common paths:
    - `C:\opencv\build\x64\vc16\bin\opencv_world4130.dll`
    - `C:\opencv\build\x64\vc15\bin\opencv_world4130.dll`
- [ ] Alternative: Copy from `target\release\` after build

### 3. Required Files Checklist
Verify these files exist and will be included:

**Root Directory:**
- [ ] `hdr-merge-master.exe` (built executable)
- [ ] `opencv_world4130.dll` (OpenCV runtime)
- [ ] `README.md` (user documentation)

**blender/ Directory:**
- [ ] `blender/HDR_Merge.blend` (Blender project file)
- [ ] `blender/blender_merge.py` (Python merge script)

**Optional (if applicable):**
- [ ] `icons/icon.ico` (application icon - embedded in .exe)

## Release Script Execution

### Option 1: PowerShell Script (Recommended)
```powershell
.\release.ps1
```

### Option 2: Batch Script
```batch
release.bat
```

### Option 3: Manual Steps
```bash
# 1. Build release
cargo build --release

# 2. Create release directory
mkdir release
mkdir release\blender

# 3. Copy files
copy target\release\hdr-merge-master.exe release\
copy opencv_world4130.dll release\  # from OpenCV install or target dir
copy blender\HDR_Merge.blend release\blender\
copy blender\blender_merge.py release\blender\
copy README_release.md release\README.md

# 4. Copy any other required DLLs
copy target\release\*.dll release\  # except opencv_world and hdr_merge_master
```

## Post-Build Verification

### 4. File Structure Check
```
release/
├── hdr-merge-master.exe
├── opencv_world4130.dll
├── README.md
└── blender/
    ├── HDR_Merge.blend
    └── blender_merge.py
```

Verify:
- [ ] All files present
- [ ] No extra/unnecessary files
- [ ] Directory structure is correct

### 5. Functional Testing

**First Run:**
- [ ] Application starts without errors
- [ ] Setup dialog appears on first run
- [ ] Can configure Blender path
- [ ] Can configure other optional paths
- [ ] Settings save correctly
- [ ] Application icon displays correctly

**Basic Processing:**
- [ ] Can add folders
- [ ] Can select profiles
- [ ] Execute button works
- [ ] Processing completes successfully
- [ ] Output files are created in correct locations

**Merge Methods:**
- [ ] Blender merge works (when Blender configured)
- [ ] OpenCV Debevec merge works
- [ ] OpenCV Robertson merge works
- [ ] Rust merge works

**Alignment:**
- [ ] align_image_stack alignment works (when configured)
- [ ] OpenCV AlignMTB works

**Tone Mapping:**
- [ ] Luminance CLI tone mapping works (when configured)
- [ ] OpenCV tone mapping works

### 6. Documentation Review
- [ ] README.md is accurate
- [ ] Version number is correct
- [ ] Download links are valid
- [ ] Requirements are clearly stated
- [ ] Troubleshooting section is helpful

## Packaging

### 7. Create Archive
```powershell
# PowerShell
Compress-Archive -Path 'release\*' -DestinationPath 'hdr-merge-master-v1.0.0.zip'
```

Or manually:
- Right-click on `release` folder
- Send to → Compressed (zipped) folder
- Rename to `hdr-merge-master-v1.0.0.zip`

### 8. Archive Verification
- [ ] Open ZIP file
- [ ] Verify all files are included
- [ ] Verify directory structure is preserved
- [ ] Check ZIP file size is reasonable

## Distribution

### 9. Release Notes
Prepare release notes including:
- [ ] Version number (v1.0.0)
- [ ] Release date
- [ ] New features
- [ ] Bug fixes
- [ ] Breaking changes (if any)
- [ ] Known issues
- [ ] Minimum requirements

### 10. Upload/Distribution
- [ ] Upload to GitHub Releases
- [ ] Attach ZIP archive
- [ ] Paste release notes
- [ ] Tag release (v1.0.0)
- [ ] Mark as latest release (if applicable)

## Post-Release

### 11. Verification After Release
- [ ] Download from release page
- [ ] Extract to fresh directory
- [ ] Run application
- [ ] Verify everything works from clean state

### 12. Documentation Updates
- [ ] Update CHANGELOG.md
- [ ] Update version in Cargo.toml (for next version)
- [ ] Update any website/documentation

## Common Issues & Solutions

### OpenCV DLL Not Found
**Problem:** `opencv_world4130.dll` not found during release preparation

**Solutions:**
1. Check OpenCV installation: `C:\opencv\build\x64\vc16\bin\`
2. Copy from target directory: `target\release\opencv_world4130.dll`
3. Reinstall OpenCV with proper PATH configuration

### Application Won't Start
**Problem:** Missing DLL error when running from release package

**Solutions:**
1. Verify all DLLs are in release directory
2. Check Visual C++ Redistributable is installed
3. Verify OpenCV DLL version matches build

### Blender Merge Fails
**Problem:** HDR merging with Blender fails

**Solutions:**
1. Verify Blender path is configured correctly
2. Check `blender/HDR_Merge.blend` exists
3. Check `blender/blender_merge.py` exists
4. Verify Blender version compatibility

## Sign-Off

- [ ] All tests passed
- [ ] All files verified
- [ ] Documentation reviewed
- [ ] Archive created
- [ ] Release published
- [ ] Post-release verification complete

**Released by:** _________________  
**Date:** _________________  
**Version:** 1.0.0
