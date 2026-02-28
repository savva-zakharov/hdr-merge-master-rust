# libstacker (OpenCV) Alignment Setup

The application supports an alternative alignment method using **libstacker**, which uses OpenCV's ECC (Enhanced Correlation Coefficient) algorithm. This method is faster than align_image_stack and doesn't require external binaries.

## Prerequisites

To enable libstacker alignment, you need to install the following dependencies:

### 1. Install LLVM/clang

Required for OpenCV Rust bindings:

```bash
choco install llvm
```

### 2. Install CMake

Required for building OpenCV bindings:

```bash
choco install cmake
```

### 3. Install OpenCV

You have OpenCV installed at `C:\tools\opencv`. The build will use the environment variables in the `.env` file to locate it.

If you need to reinstall or use vcpkg instead:

**IMPORTANT:** You must use the `x64-windows-static-md` triplet!

```bash
# Remove incorrectly installed version (if you already installed it)
.\vcpkg remove opencv:x64-windows

# Install OpenCV with the CORRECT triplet
.\vcpkg install opencv:x64-windows-static-md
```

If you get errors about the triplet not existing, you may need to update vcpkg:
```bash
cd C:\vcpkg
git pull
.\bootstrap-vcpkg.bat
.\vcpkg install opencv:x64-windows-static-md
```

### 4. Enable libstacker in Cargo.toml

Uncomment the libstacker line in `Cargo.toml`:

```toml
[dependencies]
eframe = "0.31"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rfd = "0.15"
kamadak-exif = "0.5"
chrono = "0.4"
image = "0.25"
libstacker = "0.1"  # Uncomment this line
```

### 5. Set Environment Variables

A `.env` file is included with paths for `C:\tools\opencv`. If your installation is elsewhere, adjust the paths:

```env
OPENCV_INCLUDE_PATHS=C:\tools\opencv\build\include
OPENCV_LINK_PATHS=C:\tools\opencv\build\x64\vc16\lib
OPENCV_LINK_LIBS=opencv_world4130
```

Alternatively, set them as system environment variables or use `setx`:

```cmd
setx OPENCV_INCLUDE_PATHS "C:\tools\opencv\build\include"
setx OPENCV_LINK_PATHS "C:\tools\opencv\build\x64\vc16\lib"
setx OPENCV_LINK_LIBS "opencv_world4130"
```

### 6. Rebuild the application

```bash
cargo build --release
```

## Usage

Once libstacker is enabled, you can use OpenCV alignment by:

1. Opening the application
2. Going to **Setup** (gear icon)
3. Enabling **"Use OpenCV"** checkbox
4. Saving the configuration

The alignment will now use libstacker's ECC algorithm instead of align_image_stack.

## Benefits of libstacker

- **Faster** - Modern OpenCV algorithms
- **No external binaries** - Pure Rust bindings to OpenCV
- **ECC algorithm** - Specifically designed for exposure differences (perfect for HDR brackets)
- **KeyPoint matching** - Alternative for challenging alignments (future feature)

## Troubleshooting

### "Could not find OpenCV"

Make sure OpenCV is properly installed and the `OpenCV_DIR` environment variable is set.

### "pkg-config not found"

Install pkg-config:
```bash
choco install pkgconfiglite
```

Add to PATH: `C:\Program Files\pkgconfiglite\bin`

### Build fails with clang errors

Make sure LLVM is installed:
```bash
choco install llvm
```

Restart your terminal after installation.
