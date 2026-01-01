#!/bin/bash
# Rustloader Standalone macOS App Builder
# Fixes launch issue + bundles yt-dlp
# No external dependencies required for end users

set -e

APP_NAME="Rustloader"
VERSION="0.1.1"
PROJECT_DIR="/Users/hanafi/rustprojects/Rust_loader copy"

cd "$PROJECT_DIR"

echo "Building standalone Rustloader v${VERSION}"
echo "==========================================="
echo ""

echo "[1/9] Building release binary..."
cargo build --release
echo "✓ Binary compiled"

echo ""
echo "[2/9] Downloading yt-dlp..."
mkdir -p resources/bin
if [ ! -f "resources/bin/yt-dlp" ] || [ "$1" = "--force-download" ]; then
    curl -sL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos" -o resources/bin/yt-dlp
    chmod +x resources/bin/yt-dlp
    echo "✓ yt-dlp downloaded"
else
    echo "✓ yt-dlp already exists (use --force-download to re-download)"
fi
echo "  Version: $(resources/bin/yt-dlp --version 2>/dev/null || echo 'check failed')"

echo ""
echo "[3/9] Creating bundle structure..."
rm -rf "$APP_NAME.app"
mkdir -p "$APP_NAME.app/Contents/MacOS"
mkdir -p "$APP_NAME.app/Contents/Resources/bin"
echo "✓ Bundle structure created"

echo ""
echo "[4/9] Copying binaries..."
cp target/release/rustloader "$APP_NAME.app/Contents/MacOS/rustloader_bin"
cp resources/bin/yt-dlp "$APP_NAME.app/Contents/Resources/bin/"
echo "✓ Binaries copied"

echo ""
echo "[5/9] Creating launcher script..."
cat > "$APP_NAME.app/Contents/MacOS/$APP_NAME" << 'EOF'
#!/bin/bash
# Rustloader launcher script
# Sets up PATH to include bundled yt-dlp and handles logging

DIR="$(cd "$(dirname "$0")" && pwd)"
RES="$(dirname "$DIR")/Resources"

# Add bundled yt-dlp to PATH
export PATH="$RES/bin:$PATH"

# Change to home directory (Finder launches from /)
cd "$HOME"

# Create app support directory
mkdir -p "$HOME/Library/Application Support/Rustloader"

# Set up logging
LOG="$HOME/Library/Application Support/Rustloader/launch.log"
{
    echo "=== Launch at $(date) ==="
    echo "Working directory: $(pwd)"
    echo "PATH: $PATH"
    echo "yt-dlp location: $(which yt-dlp 2>/dev/null || echo 'not found')"
    echo "yt-dlp version: $(yt-dlp --version 2>/dev/null || echo 'not available')"
    echo "---"
} >> "$LOG"

# Launch the actual app binary, redirect stderr to log
exec "$DIR/rustloader_bin" 2>> "$LOG"
EOF
chmod +x "$APP_NAME.app/Contents/MacOS/$APP_NAME"
echo "✓ Launcher script created"

echo ""
echo "[6/9] Creating Info.plist..."
cat > "$APP_NAME.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Rustloader</string>
    <key>CFBundleDisplayName</key>
    <string>Rustloader</string>
    <key>CFBundleIdentifier</key>
    <string>com.rustloader.app</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>Rustloader</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024-2026 Rustloader. All rights reserved.</string>
</dict>
</plist>
EOF
echo "✓ Info.plist created"

echo ""
echo "[7/9] Copying icon..."
if [ -f "assets/icons/AppIcon.icns" ]; then
    cp "assets/icons/AppIcon.icns" "$APP_NAME.app/Contents/Resources/"
    echo "✓ Icon copied"
else
    echo "⚠ Warning: AppIcon.icns not found, app will use default icon"
fi

# Create PkgInfo
echo "APPL????" > "$APP_NAME.app/Contents/PkgInfo"

echo ""
echo "[8/9] Code signing..."
codesign --force --deep --sign - "$APP_NAME.app"
codesign --verify --verbose "$APP_NAME.app" 2>&1 | head -2
echo "✓ App signed (ad-hoc)"

echo ""
echo "[9/9] Creating DMG..."
rm -rf dmg_temp "$APP_NAME-$VERSION.dmg"
mkdir dmg_temp
cp -R "$APP_NAME.app" dmg_temp/
ln -s /Applications dmg_temp/Applications

# Create README
cat > dmg_temp/README.txt << READMEEOF
Rustloader v${VERSION}
High-Performance Video Downloader

Installation:
  Drag Rustloader.app to Applications folder

Features:
  • yt-dlp bundled (no external dependencies!)
  • Fast multi-threaded downloads
  • Resume capability
  • Modern GUI

Requirements:
  • macOS 11.0 or later

Note: First launch may show a security warning.
Right-click the app and select "Open" to bypass it.

Support: https://github.com/ibra2000sd/rustloader
READMEEOF

hdiutil create -volname "$APP_NAME" -srcfolder dmg_temp -ov -format UDZO "$APP_NAME-$VERSION.dmg" > /dev/null
rm -rf dmg_temp
echo "✓ DMG created"

# Get file sizes
APP_SIZE=$(du -sh "$APP_NAME.app" | cut -f1)
DMG_SIZE=$(du -sh "$APP_NAME-$VERSION.dmg" | cut -f1)

echo ""
echo "==========================================="
echo "Build Complete!"
echo "==========================================="
echo ""
echo "Outputs:"
echo "  • App bundle: $APP_NAME.app ($APP_SIZE)"
echo "  • DMG: $APP_NAME-$VERSION.dmg ($DMG_SIZE)"
echo ""
echo "What's included:"
echo "  ✓ Rustloader binary"
echo "  ✓ yt-dlp (bundled, no installation needed)"
echo "  ✓ Launch logging"
echo "  ✓ Code signed (ad-hoc)"
echo ""
echo "Testing:"
echo "  Terminal: ./$APP_NAME.app/Contents/MacOS/$APP_NAME"
echo "  Finder:   open $APP_NAME.app"
echo "  Logs:     tail -f ~/Library/Application\\ Support/Rustloader/launch.log"
echo ""
echo "Distribution:"
echo "  • Upload $APP_NAME-$VERSION.dmg to GitHub releases"
echo "  • Users can install without any dependencies"
echo "  • No yt-dlp installation required!"
echo ""
echo "==========================================="
