#!/bin/bash
set -e

APP_NAME="Rustloader"
VERSION="0.1.1"
BUNDLE_ID="com.rustloader.app"
TARGET_DIR="target/release"
APP_BUNDLE="${APP_NAME}.app"
DMG_NAME="${APP_NAME}-${VERSION}.dmg"
DMG_VOLUME_NAME="${APP_NAME} ${VERSION}"

echo "Building Rustloader v${VERSION} for macOS distribution"

# Step 1: Clean and build release binary
echo "[1/7] Building release binary..."
cargo build --release

# Step 2: Create app bundle structure
echo "[2/7] Creating app bundle..."
rm -rf "${APP_BUNDLE}"
mkdir -p "${APP_BUNDLE}/Contents/"{MacOS,Resources}

# Copy binary
cp "${TARGET_DIR}/rustloader" "${APP_BUNDLE}/Contents/MacOS/rustloader"
chmod +x "${APP_BUNDLE}/Contents/MacOS/rustloader"

# Copy icon
if [ -f "assets/icons/AppIcon.icns" ]; then
    cp "assets/icons/AppIcon.icns" "${APP_BUNDLE}/Contents/Resources/AppIcon.icns"
else
    echo "Warning: AppIcon.icns not found, app will use default icon"
fi

# Copy Info.plist
cat > "${APP_BUNDLE}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>rustloader</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024-2026 Rustloader. All rights reserved.</string>
</dict>
</plist>
EOF

# Create PkgInfo
echo "APPL????" > "${APP_BUNDLE}/Contents/PkgInfo"

# Step 3: Code sign the application
echo "[3/7] Code signing application..."
# Ad-hoc signing for testing (use real identity for distribution)
codesign --force --deep --sign - "${APP_BUNDLE}"

# Step 4: Verify code signature
echo "[4/7] Verifying code signature..."
codesign --verify --verbose "${APP_BUNDLE}"
echo "Code signature valid"

# Step 5: Create temporary DMG staging directory
echo "[5/7] Creating DMG staging area..."
DMG_STAGING="dmg_staging"
rm -rf "${DMG_STAGING}"
mkdir -p "${DMG_STAGING}"

# Copy app to staging
cp -R "${APP_BUNDLE}" "${DMG_STAGING}/"

# Create Applications symlink
ln -s /Applications "${DMG_STAGING}/Applications"

# Create README
cat > "${DMG_STAGING}/README.txt" << EOF
Rustloader v${VERSION}
High-Performance Video Downloader

Installation:
1. Drag Rustloader.app to the Applications folder
2. Open Rustloader from Applications or Launchpad
3. Install yt-dlp if not already installed:
   - Using pip: pip install yt-dlp
   - Using Homebrew: brew install yt-dlp

Requirements:
- macOS 11.0 or later
- yt-dlp (for video extraction)

For more information, visit:
https://github.com/ibra2000sd/rustloader
EOF

# Step 6: Create DMG
echo "[6/7] Creating DMG..."
rm -f "${DMG_NAME}"

# Create temporary RW DMG
hdiutil create -volname "${DMG_VOLUME_NAME}" \
    -srcfolder "${DMG_STAGING}" \
    -ov -format UDRW \
    temp.dmg

# Mount and customize
device=$(hdiutil attach -readwrite -noverify -noautoopen "temp.dmg" | \
         egrep '^/dev/' | sed 1q | awk '{print $1}')

# Set window properties (optional, requires osascript)
sleep 2

# Detach
hdiutil detach "${device}"

# Convert to compressed read-only DMG
hdiutil convert temp.dmg -format UDZO -o "${DMG_NAME}"
rm -f temp.dmg

# Step 7: Clean up
echo "[7/7] Cleaning up..."
rm -rf "${DMG_STAGING}"

# Summary
echo ""
echo "=========================================="
echo "Build Complete!"
echo "=========================================="
echo "Application: ${APP_BUNDLE}"
echo "DMG:         ${DMG_NAME}"
echo ""
echo "To test the app bundle:"
echo "  open ${APP_BUNDLE}"
echo ""
echo "To test the DMG:"
echo "  open ${DMG_NAME}"
echo ""
echo "Distribution checklist:"
echo "  ✓ Binary built in release mode"
echo "  ✓ App bundle created"
echo "  ✓ Code signed (ad-hoc)"
echo "  ✓ DMG created"
echo ""
echo "For App Store or notarized distribution:"
echo "  1. Get Apple Developer ID certificate"
echo "  2. Re-sign with: codesign --force --deep --sign 'Developer ID Application: Your Name' ${APP_BUNDLE}"
echo "  3. Notarize the app with Apple"
echo "=========================================="
