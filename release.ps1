# HDR Merge Master - Release Preparation Script
# PowerShell version

$ErrorActionPreference = "Stop"

Write-Host "========================================"
Write-Host "HDR Merge Master - Release Preparation"
Write-Host "========================================"
Write-Host ""

# Configuration
$ReleaseDir = "release"
$BuildConfig = "Release"
$AppName = "hdr-merge-master"
$Version = "1.0.0"

# Clean and create release directory
Write-Host "[1/7] Creating release directory..."
if (Test-Path $ReleaseDir) {
    Remove-Item -Recurse -Force $ReleaseDir
}
New-Item -ItemType Directory -Path $ReleaseDir | Out-Null
New-Item -ItemType Directory -Path "$ReleaseDir\blender" | Out-Null

# Build the application in release mode
Write-Host "[2/7] Building application in $BuildConfig mode..."
cargo build --$BuildConfig
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Build failed!" -ForegroundColor Red
    exit 1
}

# Copy the main executable
Write-Host "[3/7] Copying executable..."
Copy-Item "target\$BuildConfig\$AppName.exe" "$ReleaseDir\" -Force
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to copy executable!" -ForegroundColor Red
    exit 1
}

# Copy blender directory
Write-Host "[4/7] Copying Blender files..."
Copy-Item "blender\HDR_Merge.blend" "$ReleaseDir\blender\" -Force
Copy-Item "blender\blender_merge.py" "$ReleaseDir\blender\" -Force
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to copy Blender files!" -ForegroundColor Red
    exit 1
}

# Copy OpenCV DLL
Write-Host "[5/7] Copying OpenCV DLL..."
$OpenCvDll = "opencv_world4130.dll"
$DllFound = $false

# Try to find in target directory
$DllPath = Get-ChildItem -Path "target\$BuildConfig" -Filter "opencv_world*.dll" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($DllPath) {
    Copy-Item $DllPath.FullName "$ReleaseDir\$OpenCvDll" -Force
    Write-Host "  Found: $($DllPath.FullName)"
    $DllFound = $true
}

# If not found, try common OpenCV installation paths
if (-not $DllFound) {
    $CommonPaths = @(
        "C:\opencv\build\x64\vc16\bin\$OpenCvDll",
        "C:\opencv\build\x64\vc15\bin\$OpenCvDll",
        "C:\Program Files\opencv\build\x64\vc16\bin\$OpenCvDll"
    )
    
    foreach ($Path in $CommonPaths) {
        if (Test-Path $Path) {
            Copy-Item $Path "$ReleaseDir\$OpenCvDll" -Force
            Write-Host "  Found: $Path"
            $DllFound = $true
            break
        }
    }
}

if (-not $DllFound) {
    Write-Host "WARNING: OpenCV DLL not found automatically!" -ForegroundColor Yellow
    Write-Host "Please copy $OpenCvDll to the release directory manually."
    Write-Host "Common locations:"
    Write-Host "  - C:\opencv\build\x64\vc16\bin\"
    Write-Host "  - Check your OpenCV installation directory"
}

# Copy other required DLLs
Write-Host "[6/7] Copying additional dependencies..."
$ExcludeList = @("opencv_world*.dll", "$AppName.exe")
Get-ChildItem "target\$BuildConfig\*.dll" | Where-Object {
    $ExcludeList -notcontains $_.Name
} | Copy-Item -Destination "$ReleaseDir\" -Force

# Copy README
Write-Host "[7/7] Copying documentation..."
Copy-Item "README_release.md" "$ReleaseDir\README.md" -Force

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Release preparation complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Release directory: $ReleaseDir"
Write-Host ""
Write-Host "Contents:"
Get-ChildItem $ReleaseDir -Recurse -File | Select-Object FullName | Format-Table -AutoSize
Write-Host ""

# Check for OpenCV DLL
if (Test-Path "$ReleaseDir\$OpenCvDll") {
    Write-Host "[OK] $OpenCvDll is present" -ForegroundColor Green
} else {
    Write-Host "[MISSING] $OpenCvDll must be added before distribution!" -ForegroundColor Red
}

Write-Host ""
Write-Host "Next steps:"
Write-Host "1. Verify $OpenCvDll is present in the release directory"
Write-Host "2. Test the application: .\$ReleaseDir\$AppName.exe"
Write-Host "3. Create a ZIP archive:"
Write-Host "   powershell -Command `"Compress-Archive -Path '$ReleaseDir\*' -DestinationPath '$AppName-v$Version.zip'`""
Write-Host ""
