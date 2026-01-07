#!/bin/bash
set -e

VERSION="${1:-0.8.0}"
DIST_DIR="dist/v${VERSION}"

echo "🔨 Building Rustloader v${VERSION} for all platforms..."

mkdir -p "$DIST_DIR"

# macOS ARM64
echo "📦 Building macOS ARM64..."
cargo build --release --target aarch64-apple-darwin
strip target/aarch64-apple-darwin/release/rustloader
tar -czf "$DIST_DIR/rustloader-v${VERSION}-macos-arm64.tar.gz" \
    -C target/aarch64-apple-darwin/release rustloader

# macOS x86_64
echo "📦 Building macOS x86_64..."
cargo build --release --target x86_64-apple-darwin
strip target/x86_64-apple-darwin/release/rustloader
tar -czf "$DIST_DIR/rustloader-v${VERSION}-macos-x86_64.tar.gz" \
    -C target/x86_64-apple-darwin/release rustloader

# Generate checksums
echo "🔐 Generating checksums..."
cd "$DIST_DIR"
shasum -a 256 *.tar.gz > SHA256SUMS.txt
cat SHA256SUMS.txt

echo "✅ Build complete! Artifacts in $DIST_DIR"
