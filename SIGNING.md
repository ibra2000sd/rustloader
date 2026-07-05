# Signing & notarizing Rustloader.app (macOS)

The bundle produced by `scripts/build-macos-app.sh` is **unsigned**. This
documents the exact steps to code-sign and notarize it once a paid
**Apple Developer** account and a **Developer ID Application** certificate
are available. Do not run any of this without them.

Until then, a technical user bypasses Gatekeeper with right-click → Open, or:

```sh
xattr -dr com.apple.quarantine /Applications/Rustloader.app
```

## What's in the bundle (signing-relevant)

| Path | What it is |
|---|---|
| `Contents/MacOS/Rustloader` | the Rust binary (universal, CFBundleExecutable) |
| `Contents/Resources/bin/deno` | JS runtime — V8 **JIT** |
| `Contents/Resources/bin/ffmpeg`, `ffprobe` | static binaries |
| `Contents/Resources/bin/yt-dlp_dir/` | PyInstaller onedir tree (~100 Mach-O dylibs/so) |
| `Contents/Resources/bin/yt-dlp` | symlink into the onedir tree |

## 1. Sign (inside-out, hardened runtime)

Sign nested Mach-O binaries first, then the bundle. `--deep` exists but Apple
discourages it for bundles this shape; explicit inside-out signing is the
reliable route. All with the same Developer ID + secure timestamp:

```sh
ID="Developer ID Application: <Name> (<TEAMID>)"

# every Mach-O inside the yt-dlp onedir tree
find Rustloader.app/Contents/Resources/bin/yt-dlp_dir -type f \
    \( -name "*.dylib" -o -name "*.so" -o -perm +111 \) \
    -exec sh -c 'file -b "$1" | grep -q Mach-O' _ {} \; \
    -exec codesign --force --options runtime --timestamp --sign "$ID" {} \;

# the standalone tools (entitlements per component, see below)
codesign --force --options runtime --timestamp --sign "$ID" \
    --entitlements deno.entitlements Rustloader.app/Contents/Resources/bin/deno
codesign --force --options runtime --timestamp --sign "$ID" \
    Rustloader.app/Contents/Resources/bin/ffmpeg \
    Rustloader.app/Contents/Resources/bin/ffprobe
codesign --force --options runtime --timestamp --sign "$ID" \
    Rustloader.app/Contents/MacOS/Rustloader

# finally the bundle itself
codesign --force --options runtime --timestamp --sign "$ID" Rustloader.app
codesign --verify --deep --strict --verbose=2 Rustloader.app
```

(The one-shot variant the docs often show —
`codesign --deep --force --options runtime --sign "$ID" Rustloader.app` —
can work, but it applies one entitlements set to everything and has known
ordering pitfalls with nested trees like PyInstaller's.)

### Likely-needed entitlements (validate during the first notarization)

- **deno** (`deno.entitlements`): V8 JIT under hardened runtime needs
  `com.apple.security.cs.allow-jit` (possibly also
  `com.apple.security.cs.allow-unsigned-executable-memory` on older deno).
- **yt-dlp (PyInstaller)**: commonly needs
  `com.apple.security.cs.allow-dyld-environment-variables` and
  `com.apple.security.cs.disable-library-validation` — PyInstaller loads its
  own dylibs at runtime. If everything is signed with the same Team ID,
  library validation may pass without the latter; test.
- **Rustloader / ffmpeg / ffprobe**: none expected.

### Resolved: no shell-script launcher anymore

Earlier bundles pointed `CFBundleExecutable` at a POSIX shell launcher that
prepended `Resources/bin` to PATH before exec'ing the real binary — a shape
notarization can object to (`--options runtime` only applies to Mach-O).
That PATH setup now lives in `main.rs::setup_bundled_tools_path()` and
`CFBundleExecutable` points straight at the Rust binary
(`Contents/MacOS/Rustloader`), so every executable in the bundle is a
Mach-O and signs with the hardened runtime normally.

## 2. Notarize + staple

```sh
# one-time: store App Store Connect credentials (app-specific password)
xcrun notarytool store-credentials "AC_PROFILE" \
    --apple-id "<apple-id-email>" --team-id "<TEAMID>" --password "<app-specific-pw>"

# build the dmg from the SIGNED app (re-run the dmg step or the script's [6/7])
xcrun notarytool submit dist/Rustloader-<version>.dmg \
    --keychain-profile "AC_PROFILE" --wait

xcrun stapler staple dist/Rustloader-<version>.dmg
xcrun stapler staple Rustloader.app   # if distributing the .app directly too
spctl -a -t open --context context:primary-signature -v dist/Rustloader-<version>.dmg
```

Notes:
- Notarization requires the hardened runtime (`--options runtime`) and
  timestamps on every signed Mach-O; the first submission's log
  (`xcrun notarytool log <id> --keychain-profile AC_PROFILE`) lists any
  binary it rejects — fix entitlements/signing for those and resubmit.
- Sign first, then build the .dmg, then notarize the .dmg; stapling makes
  the first launch work offline.
