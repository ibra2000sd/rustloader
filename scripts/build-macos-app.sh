#!/bin/bash
# Build a standalone macOS Rustloader.app + drag-to-install .dmg.
#
# Layout (matches what the code expects at runtime):
#   Rustloader.app/Contents/MacOS/rustloader       <- launcher script (CFBundleExecutable)
#   Rustloader.app/Contents/MacOS/rustloader-bin   <- the real release binary
#   Rustloader.app/Contents/Resources/bin/         <- bundled yt-dlp, ffmpeg, ffprobe, deno
#
# Why a launcher script: the extractor resolves the bundled yt-dlp itself
# (src/utils/platform.rs::ytdlp_path checks Contents/Resources/bin/), but the
# download engine spawns `yt-dlp` by name (src/downloader/engine.rs) and yt-dlp
# in turn finds `ffmpeg` on PATH. Finder launches apps with a minimal PATH, so
# the launcher prepends Contents/Resources/bin before exec'ing the binary.
#
# Output: dist/Rustloader.app and dist/Rustloader-<version>.dmg
# The .app is NOT signed or notarized — first launch needs right-click > Open
# (or `xattr -dr com.apple.quarantine Rustloader.app`).
#
# Dependency binaries are cached in target/bundle-deps/. Pre-place files there
# to skip the downloads (yt-dlp_macos.zip onedir tree, ffmpeg, ffprobe).
# ffmpeg/ffprobe come from evermeet.cx static builds (x86_64); yt-dlp is the
# official onedir build (yt-dlp_macos.zip), NOT the onefile yt-dlp_macos
# binary: onefile re-extracts itself on every run, and on macOS each freshly
# extracted library re-pays the Gatekeeper first-run scan — measured at ~55s
# PER yt-dlp invocation on an Intel Mac. The onedir build pays that scan once
# per installed file and then starts in ~1s. Resources/bin/yt-dlp is a
# relative symlink into the onedir tree so both the app's bundled-path lookup
# and the launcher's PATH find it under the expected name.
#
# Deno (single static binary, MIT) is bundled as yt-dlp's JavaScript runtime
# (B-DL-009): YouTube's web client — which yt-dlp selects whenever cookies
# are configured — needs a JS runtime to solve the n-challenge, and a
# Finder-launched app has no node/deno on PATH. The launcher's PATH-prepend
# makes the bundled deno visible to yt-dlp AND to the app's own
# depcheck::has_js_runtime() startup check.

set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME="Rustloader"
VERSION="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
DEPS="target/bundle-deps"
APP="dist/$APP_NAME.app"
DMG="dist/$APP_NAME-$VERSION.dmg"

echo "Building $APP_NAME v$VERSION standalone app"

echo "[1/6] cargo build --release"
cargo build --release

echo "[2/6] fetching bundled dependencies (cache: $DEPS)"
mkdir -p "$DEPS"
if [ ! -d "$DEPS/ytdlp_onedir" ]; then
    curl -fsSL -o "$DEPS/yt-dlp_macos.zip" \
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos.zip"
    unzip -o -q "$DEPS/yt-dlp_macos.zip" -d "$DEPS/ytdlp_onedir"
    rm "$DEPS/yt-dlp_macos.zip"
fi
for tool in ffmpeg ffprobe; do
    if [ ! -f "$DEPS/$tool" ]; then
        url="https://evermeet.cx/ffmpeg/getrelease/zip"
        [ "$tool" = "ffprobe" ] && url="https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip"
        curl -fsSL -o "$DEPS/$tool.zip" "$url"
        unzip -o -q "$DEPS/$tool.zip" -d "$DEPS"
        rm "$DEPS/$tool.zip"
    fi
done
if [ ! -f "$DEPS/deno" ]; then
    curl -fsSL -o "$DEPS/deno.zip" \
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-apple-darwin.zip"
    unzip -o -q "$DEPS/deno.zip" -d "$DEPS"
    rm "$DEPS/deno.zip"
fi
chmod +x "$DEPS/ytdlp_onedir/yt-dlp_macos" "$DEPS/ffmpeg" "$DEPS/ffprobe" "$DEPS/deno"
echo "  yt-dlp  $("$DEPS/ytdlp_onedir/yt-dlp_macos" --version)"
echo "  ffmpeg  $("$DEPS/ffmpeg" -version | head -1 | awk '{print $3}')"
echo "  deno    $("$DEPS/deno" --version | head -1 | awk '{print $2}')"

echo "[3/6] assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/bin"

cp target/release/rustloader "$APP/Contents/MacOS/rustloader-bin"
cp -R "$DEPS/ytdlp_onedir" "$APP/Contents/Resources/bin/yt-dlp_dir"
ln -s "yt-dlp_dir/yt-dlp_macos" "$APP/Contents/Resources/bin/yt-dlp"
cp "$DEPS/ffmpeg" "$DEPS/ffprobe" "$DEPS/deno" "$APP/Contents/Resources/bin/"
cp assets/icons/AppIcon.icns "$APP/Contents/Resources/"

# Launcher: put the bundled tools on PATH, then exec the real binary.
cat > "$APP/Contents/MacOS/rustloader" << 'EOF'
#!/bin/sh
DIR="$(cd "$(dirname "$0")" && pwd)"
export PATH="$DIR/../Resources/bin:$PATH"
exec "$DIR/rustloader-bin" "$@"
EOF
chmod +x "$APP/Contents/MacOS/rustloader" "$APP/Contents/MacOS/rustloader-bin" \
    "$APP/Contents/Resources/bin/"*

cat > "$APP/Contents/Info.plist" << EOF
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
    <string>rustloader</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
</dict>
</plist>
EOF
echo "APPL????" > "$APP/Contents/PkgInfo"

echo "[4/6] verifying bundle layout"
"$APP/Contents/MacOS/rustloader-bin" --version
test -x "$APP/Contents/Resources/bin/yt-dlp"
test -x "$APP/Contents/Resources/bin/ffmpeg"
test -x "$APP/Contents/Resources/bin/deno"

echo "[5/6] creating $DMG"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" > /dev/null
rm -rf "$STAGE"

echo "[6/6] done"
du -sh "$APP" "$DMG"
