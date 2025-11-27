#!/bin/bash
# Rustloader v0.1.1 - Release Packaging Script

set -e

BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m'

VERSION="0.1.1"
PLATFORM="macos"
PACKAGE_NAME="rustloader-v${VERSION}-${PLATFORM}"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}📦 CREATING RELEASE PACKAGE${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Change to project directory
cd "/Users/hanafi/rustprojects/Rust_loader copy"

# Create dist directory
echo "Creating distribution directory..."
mkdir -p dist/${PACKAGE_NAME}

# Copy binary
echo "Copying binary..."
cp target/release/rustloader dist/${PACKAGE_NAME}/
strip dist/${PACKAGE_NAME}/rustloader  # Remove debug symbols

# Copy documentation
echo "Copying documentation..."
cp README.md dist/${PACKAGE_NAME}/
cp CHANGELOG.md dist/${PACKAGE_NAME}/
cp RELEASE_NOTES.md dist/${PACKAGE_NAME}/
cp LICENSE dist/${PACKAGE_NAME}/ 2>/dev/null || echo "LICENSE not found (create if needed)"

# Create installation script
echo "Creating installation script..."
cat > dist/${PACKAGE_NAME}/install.sh << 'INSTALL_EOF'
#!/bin/bash
# Rustloader Installation Script

echo "Installing Rustloader..."

# Determine installation location
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

# Copy binary
cp rustloader "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/rustloader"

echo ""
echo "✅ Rustloader installed to: $INSTALL_DIR/rustloader"
echo ""
echo "Add to PATH (if not already):"
echo "  export PATH=\"$HOME/.local/bin:$PATH\""
echo ""
echo "Or run directly:"
echo "  $INSTALL_DIR/rustloader"
echo ""
echo "To uninstall:"
echo "  rm $INSTALL_DIR/rustloader"
INSTALL_EOF

chmod +x dist/${PACKAGE_NAME}/install.sh

# Create quick start guide
echo "Creating quick start guide..."
cat > dist/${PACKAGE_NAME}/QUICKSTART.md << 'QUICKSTART_EOF'
# Rustloader v0.1.1 - Quick Start Guide

## Installation

### Method 1: Install to PATH
```bash
cd rustloader-v0.1.1-macos
./install.sh
```

### Method 2: Run Directly
```bash
cd rustloader-v0.1.1-macos
./rustloader
```

## First Steps

1. **Install yt-dlp** (required):
   ```bash
   pip3 install yt-dlp
   ```
   Or visit: https://github.com/yt-dlp/yt-dlp

2. **Launch Rustloader**:
   ```bash
   ./rustloader
   ```

3. **Download a video**:
   - Paste YouTube URL
   - Click "Download"
   - Watch progress

## Controls

- **Pause**: Stop download temporarily
- **Resume**: Continue from pause point
- **Cancel**: Remove from queue
- **Settings**: Configure download location

## Troubleshooting

**"yt-dlp not found"**
→ Install yt-dlp: `pip3 install yt-dlp`

**"Permission denied"**
→ Make executable: `chmod +x rustloader`

**"Application can't be opened" (macOS)**
→ Right-click → Open → Confirm

## Support

- Issues: [GitHub Issues]
- Email: [Email]
- Documentation: See RELEASE_NOTES.md

**Version**: 0.1.1  
**Platform**: macOS  
**Date**: November 23, 2025
QUICKSTART_EOF

# Create archive
echo "Creating tar.gz archive..."
cd dist
tar -czf ${PACKAGE_NAME}.tar.gz ${PACKAGE_NAME}/

# Generate checksums
echo "Generating checksums..."
shasum -a 256 ${PACKAGE_NAME}.tar.gz > SHA256SUMS.txt

# Show results
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✅ PACKAGE CREATED SUCCESSFULLY${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Package: dist/${PACKAGE_NAME}.tar.gz"
echo "Size: $(ls -lh ${PACKAGE_NAME}.tar.gz | awk '{print $5}')"
echo "SHA256: $(cat SHA256SUMS.txt)"
echo ""
echo "Contents:"
ls -lh ${PACKAGE_NAME}/
echo ""
echo "Verify checksum:"
echo "  shasum -a 256 -c SHA256SUMS.txt"
echo ""
echo "Upload to GitHub:"
echo "  1. Go to Releases page"
echo "  2. Click 'Create new release'"
echo "  3. Tag: v0.1.1"
echo "  4. Upload: ${PACKAGE_NAME}.tar.gz"
echo "  5. Upload: SHA256SUMS.txt"
echo "  6. Paste RELEASE_NOTES.md content"
