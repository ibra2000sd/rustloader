#!/bin/bash
# Build a standalone UNIVERSAL (x86_64 + arm64) macOS Rustloader.app + .dmg.
#
# Layout (matches what the code expects at runtime):
#   Rustloader.app/Contents/MacOS/Rustloader       <- universal release binary
#                                                     (lipo), CFBundleExecutable
#   Rustloader.app/Contents/Resources/bin/         <- bundled yt-dlp, ffmpeg, ffprobe, deno
#
# The Rust binary is the CFBundleExecutable directly — no shell launcher. The
# PATH-prepend the launcher used to do (Finder launches apps with a minimal
# PATH; yt-dlp is spawned by name and finds ffmpeg/deno on PATH) lives in
# main.rs::setup_bundled_tools_path(), which puts Contents/Resources/bin on
# PATH before anything spawns. The binary is named "Rustloader" (capital R)
# because macOS derives the app-menu title (About/Quit …) from the process
# name.
#
# Universal strategy per binary:
#   rustloader      cargo build --release for x86_64-apple-darwin AND
#                   aarch64-apple-darwin (both rustup targets required),
#                   then `lipo -create`.
#   yt-dlp          the official onedir build (yt-dlp_macos.zip) is already
#                   universal2 — every Mach-O in it carries both slices
#                   (verified exhaustively; the script re-asserts the main
#                   binary). Kept as-is.
#   ffmpeg/ffprobe  static per-arch builds from https://ffmpeg.martin-riedl.de
#                   (same version for both arches), then `lipo -create`.
#                   (evermeet.cx, used previously, publishes x86_64 only.)
#   deno            official per-arch release zips, then `lipo -create`.
#
# Output: dist/Rustloader.app and dist/Rustloader-<version>.dmg
# The .app is NOT signed or notarized — first launch needs right-click > Open
# (or `xattr -dr com.apple.quarantine Rustloader.app`). For the signing +
# notarization procedure (Developer ID required), see SIGNING.md.
#
# Dependency binaries are cached in target/bundle-deps/. Pre-place the final
# universal files under target/bundle-deps/universal/ to skip downloads.
#
# yt-dlp is the onedir build, NOT the onefile yt-dlp_macos binary: onefile
# re-extracts itself on every run, and on macOS each freshly extracted library
# re-pays the Gatekeeper first-run scan — measured at ~55s PER yt-dlp
# invocation on an Intel Mac. The onedir build pays that scan once per
# installed file and then starts in ~1s. Resources/bin/yt-dlp is a relative
# symlink into the onedir tree so both the app's bundled-path lookup and the
# PATH lookup find it under the expected name.
#
# Deno (single static binary, MIT) is bundled as yt-dlp's JavaScript runtime
# (B-DL-009): YouTube's web client — which yt-dlp selects whenever cookies
# are configured — needs a JS runtime to solve the n-challenge, and a
# Finder-launched app has no node/deno on PATH. The main.rs PATH-prepend
# makes the bundled deno visible to yt-dlp AND to the app's own
# depcheck::has_js_runtime() startup check.

set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME="Rustloader"
VERSION="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
DEPS="target/bundle-deps"
UNI="$DEPS/universal"
APP="dist/$APP_NAME.app"
DMG="dist/$APP_NAME-$VERSION.dmg"

# Every Mach-O placed in the bundle must carry both slices.
assert_universal() {
    lipo -info "$1" | grep -q "x86_64" && lipo -info "$1" | grep -q "arm64" \
        || { echo "ERROR: $1 is not universal: $(lipo -info "$1")"; exit 1; }
}

echo "Building $APP_NAME v$VERSION universal standalone app"

echo "[1/7] cargo build --release (x86_64 + aarch64) + lipo"
rustup target list --installed | grep -q x86_64-apple-darwin \
    || rustup target add x86_64-apple-darwin
rustup target list --installed | grep -q aarch64-apple-darwin \
    || rustup target add aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
mkdir -p target/universal
lipo -create -output target/universal/rustloader \
    target/x86_64-apple-darwin/release/rustloader \
    target/aarch64-apple-darwin/release/rustloader
assert_universal target/universal/rustloader

echo "[2/7] fetching bundled dependencies (cache: $DEPS)"
mkdir -p "$DEPS" "$UNI"
if [ ! -d "$DEPS/ytdlp_onedir" ]; then
    curl -fsSL -o "$DEPS/yt-dlp_macos.zip" \
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos.zip"
    unzip -o -q "$DEPS/yt-dlp_macos.zip" -d "$DEPS/ytdlp_onedir"
    rm "$DEPS/yt-dlp_macos.zip"
fi
if [ ! -f "$UNI/deno" ]; then
    for arch in x86_64 aarch64; do
        curl -fsSL -o "$DEPS/deno-$arch.zip" \
            "https://github.com/denoland/deno/releases/latest/download/deno-$arch-apple-darwin.zip"
        mkdir -p "$DEPS/deno-$arch"
        unzip -o -q "$DEPS/deno-$arch.zip" -d "$DEPS/deno-$arch"
        rm "$DEPS/deno-$arch.zip"
    done
    lipo -create -output "$UNI/deno" "$DEPS/deno-x86_64/deno" "$DEPS/deno-aarch64/deno"
fi
for tool in ffmpeg ffprobe; do
    if [ ! -f "$UNI/$tool" ]; then
        for arch in amd64 arm64; do
            curl -fsSL -o "$DEPS/$tool-$arch.zip" \
                "https://ffmpeg.martin-riedl.de/redirect/latest/macos/$arch/release/$tool.zip"
            mkdir -p "$DEPS/$tool-$arch"
            unzip -o -q "$DEPS/$tool-$arch.zip" -d "$DEPS/$tool-$arch"
            rm "$DEPS/$tool-$arch.zip"
        done
        lipo -create -output "$UNI/$tool" "$DEPS/$tool-amd64/$tool" "$DEPS/$tool-arm64/$tool"
    fi
done
chmod +x "$DEPS/ytdlp_onedir/yt-dlp_macos" "$UNI/deno" "$UNI/ffmpeg" "$UNI/ffprobe"
echo "  yt-dlp  $("$DEPS/ytdlp_onedir/yt-dlp_macos" --version)"
echo "  ffmpeg  $("$UNI/ffmpeg" -version | head -1 | awk '{print $3}')"
echo "  deno    $("$UNI/deno" --version | head -1 | awk '{print $2}')"

echo "[3/7] assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/bin"

cp target/universal/rustloader "$APP/Contents/MacOS/Rustloader"
cp -R "$DEPS/ytdlp_onedir" "$APP/Contents/Resources/bin/yt-dlp_dir"
ln -s "yt-dlp_dir/yt-dlp_macos" "$APP/Contents/Resources/bin/yt-dlp"
cp "$UNI/ffmpeg" "$UNI/ffprobe" "$UNI/deno" "$APP/Contents/Resources/bin/"
cp assets/icons/AppIcon.icns "$APP/Contents/Resources/"
chmod +x "$APP/Contents/MacOS/Rustloader" "$APP/Contents/Resources/bin/"*

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
    <string>Rustloader</string>
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

echo "[4/7] asserting every bundled tool is universal"
assert_universal "$APP/Contents/MacOS/Rustloader"
assert_universal "$APP/Contents/Resources/bin/deno"
assert_universal "$APP/Contents/Resources/bin/ffmpeg"
assert_universal "$APP/Contents/Resources/bin/ffprobe"
assert_universal "$APP/Contents/Resources/bin/yt-dlp_dir/yt-dlp_macos"

echo "[5/7] verifying bundle layout (native slice)"
"$APP/Contents/MacOS/Rustloader" --version
test -x "$APP/Contents/Resources/bin/yt-dlp"

echo "[6/7] creating $DMG"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" > /dev/null
rm -rf "$STAGE"

echo "[7/7] done"
du -sh "$APP" "$DMG"
