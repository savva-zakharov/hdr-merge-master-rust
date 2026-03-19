@echo off
REM HDR Merge Master - Release Preparation Script
REM This script prepares a release build with all required dependencies

setlocal enabledelayedexpansion

echo ========================================
echo HDR Merge Master - Release Preparation
echo ========================================
echo.

REM Configuration
set RELEASE_DIR=release
set BUILD_CONFIG=release
set APP_NAME=hdr-merge-master

REM Clean and create release directory
echo [1/6] Creating release directory...
if exist "%RELEASE_DIR%" rmdir /s /q "%RELEASE_DIR%"
mkdir "%RELEASE_DIR%"
mkdir "%RELEASE_DIR%\blender"

REM Build the application in release mode
echo [2/6] Building application in %BUILD_CONFIG% mode...
cargo build --%BUILD_CONFIG%
if errorlevel 1 (
    echo ERROR: Build failed!
    exit /b 1
)

REM Copy the main executable
echo [3/6] Copying executable...
copy "target\%BUILD_CONFIG%\%APP_NAME%.exe" "%RELEASE_DIR%\" > nul
if errorlevel 1 (
    echo ERROR: Failed to copy executable!
    exit /b 1
)

REM Copy blender directory
echo [4/6] Copying Blender files...
copy "blender\HDR_Merge.blend" "%RELEASE_DIR%\blender\" > nul
copy "blender\blender_merge.py" "%RELEASE_DIR%\blender\" > nul
if errorlevel 1 (
    echo ERROR: Failed to copy Blender files!
    exit /b 1
)

REM Copy OpenCV DLL
echo [5/6] Copying OpenCV DLL...
REM The OpenCV DLL should be in the target directory after build
REM or we need to find it in the system
if exist "target\%BUILD_CONFIG%\opencv_world4130.dll" (
    copy "target\%BUILD_CONFIG%\opencv_world4130.dll" "%RELEASE_DIR%\" > nul
) else (
    REM Try to find it in the output directory
    for /f "delims=" %%i in ('dir /b /s "target\%BUILD_CONFIG%\opencv_world*.dll" 2^>nul') do (
        copy "%%i" "%RELEASE_DIR%\" > nul
        goto :dll_copied
    )
    REM If not found, try common OpenCV installation paths
    if exist "C:\opencv\build\x64\vc16\bin\opencv_world4130.dll" (
        copy "C:\opencv\build\x64\vc16\bin\opencv_world4130.dll" "%RELEASE_DIR%\" > nul
        goto :dll_copied
    )
    echo WARNING: OpenCV DLL not found automatically.
    echo Please copy opencv_world4130.dll to the release directory manually.
)
:dll_copied

REM Copy other required DLLs (MSVC runtime, etc.)
echo [6/6] Copying additional dependencies...
REM Copy any other DLLs from the build directory that are needed
for %%i in (
    "target\%BUILD_CONFIG%\*.dll"
) do (
    if not "%%~ni" == "opencv_world4130" (
        if not "%%~ni" == "hdr_merge_master" (
            copy "%%i" "%RELEASE_DIR%\" > nul 2>&1
        )
    )
)

echo.
echo ========================================
echo Release preparation complete!
echo ========================================
echo.
echo Release directory: %RELEASE_DIR%
echo.
echo Contents:
dir /b "%RELEASE_DIR%"
echo.
echo Next steps:
echo 1. Verify opencv_world4130.dll is present in the release directory
echo 2. Test the application
echo 3. Create a ZIP archive: powershell -Command "Compress-Archive -Path '%RELEASE_DIR%\*' -DestinationPath '%APP_NAME%-v1.0.0.zip'"
echo.

endlocal
