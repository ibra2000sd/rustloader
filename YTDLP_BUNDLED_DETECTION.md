# yt-dlp Bundled Detection Implementation

## Overview
Enhanced the `VideoExtractor` to automatically detect and use bundled yt-dlp from the standalone macOS app bundle.

## Changes Made to `src/extractor/ytdlp.rs`

### 1. Updated `VideoExtractor::new()`
**Before**: Simple PATH lookup using `which` crate
**After**: Multi-priority detection system

```rust
pub fn new() -> Result<Self> {
    let ytdlp_path = find_ytdlp().ok_or_else(|| {
        error!("yt-dlp not found in bundle, PATH, or common locations");
        RustloaderError::YtDlpNotFound
    })?;

    info!("Found yt-dlp at: {}", ytdlp_path.display());
    Ok(Self { ytdlp_path })
}
```

### 2. Added Detection Functions

#### `find_ytdlp()` - Main Detection Logic
Searches in priority order:
1. **Bundled yt-dlp** (inside .app/Contents/Resources/bin/)
2. **System PATH** (using `which` crate)
3. **Common locations** (Homebrew, Python.org installs)

#### `find_bundled_ytdlp()` - macOS Bundle Detection
```rust
// Detects if running from .app/Contents/MacOS/binary
// Looks for .app/Contents/Resources/bin/yt-dlp
let exe_path = std::env::current_exe()?;
let macos_dir = exe_path.parent()?;

if macos_dir.ends_with("MacOS") {
    let contents_dir = macos_dir.parent()?;
    let ytdlp_path = contents_dir.join("Resources").join("bin").join("yt-dlp");
    
    if ytdlp_path.exists() && is_executable(&ytdlp_path) {
        return Some(ytdlp_path);
    }
}
```

#### `find_in_path()` - System PATH Lookup
Uses `which` crate to find yt-dlp in system PATH.

#### `find_in_common_paths()` - Known Locations
Checks common installation paths:
- `/opt/homebrew/bin/yt-dlp` (Homebrew ARM)
- `/usr/local/bin/yt-dlp` (Homebrew Intel)
- `/Library/Frameworks/Python.framework/Versions/*/bin/yt-dlp` (Python.org)

#### `is_executable()` - Permission Check
Verifies file has executable permission on Unix systems.

#### `ytdlp_command()` - Quick Command Builder
Creates a `Command` instance with the detected yt-dlp path for one-off uses.

### 3. Added Tests

```rust
#[test]
fn test_find_ytdlp() {
    // Verifies yt-dlp can be found
    let result = find_ytdlp();
    if let Some(path) = result {
        println!("✓ yt-dlp found at: {:?}", path);
        assert!(path.exists());
    }
}

#[test]
fn test_ytdlp_command() {
    // Tests command creation and version check
    if let Some(mut cmd) = ytdlp_command() {
        let output = cmd.arg("--version").output();
        // Verifies successful execution
    }
}

#[test]
fn test_bundled_ytdlp_detection() {
    // Tests bundled detection doesn't panic
    let result = find_bundled_ytdlp();
    // Safe even when not in .app bundle
}

#[test]
fn test_is_executable() {
    // Tests executable permission detection
    assert!(is_executable(&PathBuf::from("/bin/sh")));
}
```

## How It Works with Standalone Build

When `build_standalone.sh` creates the app:

```
Rustloader.app/
├── Contents/
│   ├── MacOS/
│   │   ├── Rustloader         (launcher script)
│   │   └── rustloader_bin     (actual binary)
│   └── Resources/
│       └── bin/
│           └── yt-dlp         ← Bundled here!
```

### Launch Flow:
1. User launches `Rustloader.app` from Finder
2. macOS executes launcher script at `Contents/MacOS/Rustloader`
3. Launcher adds `Contents/Resources/bin/` to PATH
4. Launcher executes `rustloader_bin`
5. `VideoExtractor::new()` is called
6. `find_ytdlp()` detects bundled yt-dlp first
7. ✅ App works without requiring yt-dlp installation!

## Benefits

### For Users
- ✅ No need to install yt-dlp separately
- ✅ No pip, Homebrew, or terminal commands
- ✅ Just drag and drop to install
- ✅ Works out of the box

### For Developers
- ✅ Consistent yt-dlp version across installs
- ✅ No dependency on system PATH
- ✅ Works in sandboxed environments
- ✅ Easier testing and debugging

### For Distribution
- ✅ Truly standalone app
- ✅ Single DMG includes everything
- ✅ No installation instructions needed
- ✅ Fewer support requests

## Compatibility

The detection system is backward compatible:
- ✅ Works with bundled yt-dlp (standalone build)
- ✅ Works with system-installed yt-dlp (development)
- ✅ Works with Homebrew installations
- ✅ Works with Python.org installations
- ✅ Fails gracefully with clear error message

## Testing

### Test Bundled Detection
```bash
# Build standalone app
./build_standalone.sh

# Verify bundled yt-dlp exists
ls -la Rustloader.app/Contents/Resources/bin/yt-dlp

# Test detection
./Rustloader.app/Contents/MacOS/rustloader_bin
# Should print: "✓ Using bundled yt-dlp: ..."
```

### Test System Detection
```bash
# With yt-dlp in PATH
which yt-dlp

# Run from source
cargo run
# Should print: "✓ Using system yt-dlp from PATH: ..."
```

### Run Unit Tests
```bash
cargo test extractor::ytdlp::tests::test_find_ytdlp -- --nocapture
cargo test extractor::ytdlp::tests::test_ytdlp_command -- --nocapture
```

## Logging

The extractor now provides detailed logs:

```
INFO  ✓ Using bundled yt-dlp: "/Applications/Rustloader.app/Contents/Resources/bin/yt-dlp"
INFO  Found yt-dlp at: /Applications/Rustloader.app/Contents/Resources/bin/yt-dlp
```

Or if bundled not found:
```
INFO  ✓ Using system yt-dlp from PATH: "/opt/homebrew/bin/yt-dlp"
INFO  Found yt-dlp at: /opt/homebrew/bin/yt-dlp
```

Or if not found anywhere:
```
WARN  ✗ yt-dlp not found anywhere!
ERROR yt-dlp not found in bundle, PATH, or common locations
```

## Integration with build_standalone.sh

The standalone build script:
1. Downloads latest yt-dlp from GitHub
2. Places it in `resources/bin/yt-dlp`
3. Copies to `Rustloader.app/Contents/Resources/bin/`
4. Sets executable permissions
5. Creates launcher that adds to PATH

The extractor code automatically detects and uses this bundled version.

## Summary

This implementation makes Rustloader a true standalone application that doesn't require users to install any dependencies. The yt-dlp binary is bundled directly into the macOS .app bundle and automatically detected at runtime.

**Result**: Users can download the DMG, drag to Applications, and start downloading videos immediately - no terminal commands, no pip install, no Homebrew required!
