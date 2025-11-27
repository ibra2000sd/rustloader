# Changelog

All notable changes to Rustloader will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-11-23

### 🔒 Security
- **CRITICAL FIX**: Eliminated RUSTSEC-2023-0071 RSA timing attack vulnerability
  - Removed vulnerable `rsa 0.9.9` crate from dependency tree
  - Switched from `rsa` to `ring` cryptography library (via rustls)
  - Changed sqlx runtime from `runtime-tokio` to `runtime-tokio-rustls`
  - Verified with `cargo tree` - no RSA vulnerability in final binary
  - **Impact**: Application no longer vulnerable to Marvin timing attacks

### 🐛 Critical Bug Fixes
- **CRITICAL FIX**: Resolved mutex deadlock risk in async operations
  - Fixed `std::sync::Mutex` held across `.await` points in `app.rs` (lines 221, 302)
  - Implemented RAII pattern: acquire lock → clone bridge → drop lock → await
  - Added `Clone` implementation to `BackendBridge` for safe async usage
  - Verified with `cargo clippy --await_holding_lock` - 0 warnings
  - **Impact**: Eliminates application freezing during concurrent video extractions

- **MAJOR FIX**: Wired up Pause/Resume/Cancel UI buttons to backend
  - Implemented 5 message handlers: PauseTask, ResumeTask, CancelTask, RemoveCompleted, ClearCompleted
  - All buttons now trigger backend operations instead of just updating UI
  - Added proper async communication between GUI and queue manager
  - **Impact**: Users can now pause/resume downloads instead of restarting

### 📦 Dependencies
- Changed `sqlx` features to use `runtime-tokio-rustls` (more secure)
- Removed unused MySQL database backend support
- Binary now uses `ring` for cryptography (safer than OpenSSL)

### 🧹 Code Quality
- Reduced clippy warnings from 59 to 39 (34% improvement)
- Improved code maintainability with better async patterns
- Enhanced error handling in GUI message handlers

### 🔧 Technical Details
- **Files Modified**: 
  - `src/gui/app.rs` - Fixed 2 deadlocks + wired 5 button handlers
  - `src/gui/integration.rs` - Added Clone trait to BackendBridge
  - `Cargo.toml` - Switched to rustls-based TLS
  - `src/database/schema.rs` - Simplified initialization

- **Testing**:
  - All unit tests passing (5/5)
  - Clippy deadlock detection: PASS (0 warnings)
  - Dependency tree verification: PASS (no vulnerable crates)
  - Binary size: 20MB (stable)

### ⚠️ Known Limitations
- Manual integration testing required for pause/resume functionality
- Performance benchmarks not yet completed
- Cross-platform builds (Windows/Linux) not yet tested
- Test coverage still low (<10%)

### 📝 Breaking Changes
None. This is a bug fix release with full backward compatibility.

---

## [0.1.0] - 2025-11-20 (Initial Release)

### Added
- Multi-threaded download engine (16 segments)
- yt-dlp integration for video extraction
- Iced-based GUI with modern interface
- SQLite database for download history
- Queue management system
- Progress tracking and monitoring
- Settings persistence
- Support for 1000+ video sites via yt-dlp

### Known Issues
- Mutex deadlock risk in concurrent operations (FIXED in v0.1.1)
- RSA security vulnerability (FIXED in v0.1.1)
- Pause/Resume buttons non-functional (FIXED in v0.1.1)

---

## Unreleased

### Planned for v0.2.0
- Performance benchmarking suite
- Cross-platform builds (Windows, Linux)
- Integration test suite
- Memory optimization (box large enum variants)
- Code quality improvements (reduce clippy warnings to <10)
- Comprehensive user documentation
- Video format selection UI
- Playlist download support

### Planned for v1.0.0
- >80% test coverage
- Production-ready performance (<3s startup, 5-10x faster than yt-dlp)
- Full cross-platform compatibility
- Localization support
- Plugin system for custom extractors
- Batch download management
- Advanced error recovery

---

## Version History

- **v0.1.1** (2025-11-23) - Critical bug fixes, security patches, beta-ready
- **v0.1.0** (2025-11-20) - Initial development release

---

**Maintainer**: Ibrahim Mohamed  
**Repository**: https://github.com/ibra2000sd/rustloader  
**License**: MIT  
**Rust Version**: 1.91.1+
# Changelog

All notable changes to Rustloader will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-11-23

### 🔒 Security
- **CRITICAL FIX**: Eliminated RUSTSEC-2023-0071 RSA timing attack vulnerability
  - Removed vulnerable `rsa 0.9.9` crate from dependency tree
  - Switched from `rsa` to `ring` cryptography library (via rustls)
  - Changed sqlx runtime from `runtime-tokio` to `runtime-tokio-rustls`
  - Verified with `cargo tree` - no RSA vulnerability in final binary
  - **Impact**: Application no longer vulnerable to Marvin timing attacks

### 🐛 Critical Bug Fixes
- **CRITICAL FIX**: Resolved mutex deadlock risk in async operations
  - Fixed `std::sync::Mutex` held across `.await` points in `app.rs` (lines 221, 302)
  - Implemented RAII pattern: acquire lock → clone bridge → drop lock → await
  - Added `Clone` implementation to `BackendBridge` for safe async usage
  - Verified with `cargo clippy --await_holding_lock` - 0 warnings
  - **Impact**: Eliminates application freezing during concurrent video extractions

- **MAJOR FIX**: Wired up Pause/Resume/Cancel UI buttons to backend
  - Implemented 5 message handlers: PauseTask, ResumeTask, CancelTask, RemoveCompleted, ClearCompleted
  - All buttons now trigger backend operations instead of just updating UI
  - Added proper async communication between GUI and queue manager
  - **Impact**: Users can now pause/resume downloads instead of restarting

### 📦 Dependencies
- Changed `sqlx` features to use `runtime-tokio-rustls` (more secure)
- Removed unused MySQL database backend support
- Binary now uses `ring` for cryptography (safer than OpenSSL)

### 🧹 Code Quality
- Reduced clippy warnings from 59 to 39 (34% improvement)
- Improved code maintainability with better async patterns
- Enhanced error handling in GUI message handlers

### 🔧 Technical Details
- **Files Modified**: 
  - `src/gui/app.rs` - Fixed 2 deadlocks + wired 5 button handlers
  - `src/gui/integration.rs` - Added Clone trait to BackendBridge
  - `Cargo.toml` - Switched to rustls-based TLS
  - `src/database/schema.rs` - Simplified initialization

- **Testing**:
  - All unit tests passing (5/5)
  - Clippy deadlock detection: PASS (0 warnings)
  - Dependency tree verification: PASS (no vulnerable crates)
  - Binary size: 20MB (stable)

### ⚠️ Known Limitations
- Manual integration testing required for pause/resume functionality
- Performance benchmarks not yet completed
- Cross-platform builds (Windows/Linux) not yet tested
- Test coverage still low (<10%)

### 📝 Breaking Changes
None. This is a bug fix release with full backward compatibility.

---

## [0.1.0] - 2025-11-20 (Initial Release)

### Added
- Multi-threaded download engine (16 segments)
- yt-dlp integration for video extraction
- Iced-based GUI with modern interface
- SQLite database for download history
- Queue management system
- Progress tracking and monitoring
- Settings persistence
- Support for 1000+ video sites via yt-dlp

### Known Issues
- Mutex deadlock risk in concurrent operations (FIXED in v0.1.1)
- RSA security vulnerability (FIXED in v0.1.1)
- Pause/Resume buttons non-functional (FIXED in v0.1.1)

---

## Unreleased

### Planned for v0.2.0
- Performance benchmarking suite
- Cross-platform builds (Windows, Linux)
- Integration test suite
- Memory optimization (box large enum variants)
- Code quality improvements (reduce clippy warnings to <10)
- Comprehensive user documentation
- Video format selection UI
- Playlist download support

### Planned for v1.0.0
- >80% test coverage
- Production-ready performance (<3s startup, 5-10x faster than yt-dlp)
- Full cross-platform compatibility
- Localization support
- Plugin system for custom extractors
- Batch download management
- Advanced error recovery

---

## Version History

- **v0.1.1** (2025-11-23) - Critical bug fixes, security patches, beta-ready
- **v0.1.0** (2025-11-20) - Initial development release

---

**Maintainer**: Ibrahim Hanafi  
**Repository**: https://github.com/ibra2000sd/rustloader  
**License**: MIT  
**Rust Version**: 1.91.1+
