# Rustloader v0.1.0 - Known Issues & Bug Tracker

**Last Updated**: November 23, 2025  
**Total Issues**: 5 (2 High, 2 Medium, 1 Low)

---

## PRIORITY CLASSIFICATION

- **P1 (Critical)**: Must fix before any release
- **P2 (High)**: Must fix before beta release
- **P3 (Medium)**: Should fix before v1.0
- **P4 (Low)**: Nice to have, technical debt

---

## OPEN ISSUES

### 🔴 BUG-001: Mutex Deadlock Risk in Video Extraction
- **Priority**: P1 (CRITICAL)
- **Severity**: HIGH
- **Component**: GUI Integration
- **File**: `src/gui/app.rs`
- **Lines**: 221, 302
- **Discovered**: 2025-11-23 (QA Testing)

**Description**:
MutexGuard is held across `.await` points during video extraction, which can cause deadlocks when multiple async operations attempt to access the same resource.

**Impact**:
Application may freeze or become unresponsive when:
- User triggers multiple video extractions simultaneously
- Network latency causes long-running async operations
- Multiple threads contend for the same bridge mutex

**Reproduction Steps**:
1. Paste video URL
2. Click "Extract Info" button
3. Immediately paste another URL and click "Extract Info" again
4. Observe potential freeze/hang

**Code Location**:
```rust
// Line 221
match self.backend_bridge.lock().unwrap().as_ref() {
    Ok(mut bridge) => bridge.extract_video_info(&url_clone).await,
    //    ^^^ MutexGuard held across await
}

// Line 302  
match self.backend_bridge.lock().unwrap().as_ref() {
    Ok(mut bridge) => bridge.start_download(...).await,
    //    ^^^ MutexGuard held across await
}
```

**Recommended Fix**:
```rust
// Extract reference before await
let bridge = match self.backend_bridge.lock().unwrap().as_ref() {
    Ok(bridge) => bridge.clone(),
    Err(e) => return Command::none(),
};
// Lock is dropped here
let result = bridge.extract_video_info(&url_clone).await;
```

**Alternative Fix**:
Use `tokio::sync::Mutex` instead of `std::sync::Mutex` for async-aware locking:
```rust
// Change field type
backend_bridge: Arc<tokio::sync::Mutex<Result<BackendBridge, String>>>

// Usage
let bridge = self.backend_bridge.lock().await;
let result = match bridge.as_ref() {
    Ok(b) => b.extract_video_info(&url_clone).await,
    Err(e) => Err(e.clone()),
};
```

**Testing Required**:
- [ ] Unit test: Verify non-blocking behavior
- [ ] Integration test: Simultaneous extractions
- [ ] Stress test: 10+ concurrent requests

**Assigned To**: TBD  
**Status**: OPEN  
**Target Release**: v0.1.1

---

### 🟡 BUG-002: RSA Security Vulnerability (RUSTSEC-2023-0071)
- **Priority**: P2 (HIGH)
- **Severity**: MEDIUM (Mitigated)
- **Component**: Dependencies (sqlx)
- **CVE**: RUSTSEC-2023-0071
- **Discovered**: 2025-11-23 (cargo audit)

**Description**:
The `rsa` crate version 0.9.9 contains a Marvin Attack vulnerability allowing potential private key recovery through timing sidechannel analysis.

**Impact**:
- **Actual Risk**: LOW (Rustloader uses SQLite, not MySQL)
- **Theoretical Risk**: MEDIUM (if MySQL were enabled)
- Security scanners flag this as a vulnerability

**Dependency Tree**:
```
rsa 0.9.9
└── sqlx-mysql 0.8.6
    └── sqlx 0.8.6
        └── rustloader 0.1.0
```

**Why This Matters**:
Even though Rustloader doesn't use MySQL connections, the vulnerability exists in compiled code and may:
1. Fail security audits in enterprise environments
2. Prevent acceptance in security-conscious organizations
3. Increase attack surface unnecessarily

**Recommended Fix**:
Disable MySQL features in `Cargo.toml`:
```toml
[dependencies]
sqlx = { 
    version = "0.8", 
    features = ["sqlite", "runtime-tokio", "chrono"],
    default-features = false 
}
```

**Verification**:
```bash
cargo audit
# Should return: 0 vulnerabilities found
```

**Effort**: 10 minutes  
**Risk of Breaking**: LOW (MySQL never used)  
**Assigned To**: TBD  
**Status**: OPEN  
**Target Release**: v0.1.1

---

### 🟡 BUG-003: Large Enum Variant Causing Stack Pressure
- **Priority**: P3 (MEDIUM)
- **Severity**: MEDIUM
- **Component**: Backend Integration
- **File**: `src/gui/integration.rs`
- **Line**: 29
- **Discovered**: 2025-11-23 (clippy warning)

**Description**:
The `ProgressUpdate` enum has a large size discrepancy between variants:
- `ExtractionComplete(VideoInfo)`: 304 bytes
- `TaskStatusChanged { ... }`: 72 bytes

This causes the entire enum to occupy 304 bytes on the stack, even when carrying smaller variants.

**Impact**:
- Increased memory usage (4x larger than necessary)
- Potential stack overflow with deep call chains
- Poor cache locality
- Unnecessary memory allocations

**Performance Calculation**:
```
Channels pass ~100 messages/second during downloads
Memory waste: (304 - 72) * 100 = 23.2 KB/s
Over 1-hour download: 83 MB wasted
```

**Code Location**:
```rust
pub enum ProgressUpdate {
    ExtractionComplete(VideoInfo), // 304 bytes
    DownloadProgress { ... },      // ~50 bytes
    DownloadComplete { ... },      // ~72 bytes
    DownloadFailed { ... },        // ~50 bytes
    TaskStatusChanged { ... },     // 72 bytes
}
```

**Recommended Fix**:
```rust
pub enum ProgressUpdate {
    ExtractionComplete(Box<VideoInfo>), // Now ~8 bytes (pointer)
    DownloadProgress { ... },
    DownloadComplete { ... },
    DownloadFailed { ... },
    TaskStatusChanged { ... },
}
```

**Changes Required**:
1. Modify `ProgressUpdate` enum definition
2. Update pattern matching: `ExtractionComplete(video_info)` → `ExtractionComplete(ref video_info)` or `*video_info`
3. Update construction: `ExtractionComplete(info)` → `ExtractionComplete(Box::new(info))`

**Testing Required**:
- [ ] All existing tests still pass
- [ ] Benchmark memory usage before/after
- [ ] Verify no performance regression

**Effort**: 1-2 hours  
**Assigned To**: TBD  
**Status**: OPEN  
**Target Release**: v0.2.0

---

### 🟡 BUG-004: Dead Code in Core Modules
- **Priority**: P2 (HIGH)
- **Severity**: MEDIUM
- **Component**: Queue Management
- **File**: `src/queue/manager.rs`
- **Lines**: 130, 162, 178, 237, 249
- **Discovered**: 2025-11-23 (dead_code warnings)

**Description**:
Core functionality methods are implemented but never called:
- `pause_task(&self, task_id: &str)`
- `resume_task(&self, task_id: &str)`
- `cancel_task(&self, task_id: &str)`
- `clear_completed(&self)`
- `remove_task(&self, task_id: &str)`

**Impact**:
- **Technical Debt**: Untested code that may contain bugs
- **User Experience**: Features advertised in GUI but non-functional
- **Architecture Confusion**: Unclear if features are abandoned or in-progress
- **Maintenance Burden**: Code that must be maintained but provides no value

**Analysis**:
Looking at `src/gui/components/download_item.rs`, pause/cancel buttons exist in UI:
```rust
button("Pause").on_press(Message::PauseTask(task.id.clone())),
button("Cancel").on_press(Message::CancelTask(task.id.clone())),
```

But these messages are never handled in `app.rs::update()`.

**Root Cause**:
Incomplete integration between GUI and backend. Implementation exists in `QueueManager` and `BackendBridge`, but message handlers in `Application` are missing or not wired up.

**Options**:
1. **Option A - Implement Fully** (Recommended)
   - Wire up pause/resume/cancel buttons in GUI
   - Implement message handlers in `app.rs`
   - Add integration tests
   - Effort: 4-6 hours

2. **Option B - Remove for Now**
   - Delete unused methods from `QueueManager`
   - Remove UI buttons
   - Document as future feature
   - Effort: 1 hour

**Recommended Fix** (Option A):
```rust
// In src/gui/app.rs::update()
Message::PauseTask(task_id) => {
    let backend = self.backend_bridge.clone();
    Command::perform(
        async move {
            backend.lock().unwrap()
                .as_ref().ok()?
                .pause_download(&task_id).await.ok()
        },
        |_| Message::None
    )
}

Message::CancelTask(task_id) => {
    let backend = self.backend_bridge.clone();
    Command::perform(
        async move {
            backend.lock().unwrap()
                .as_ref().ok()?
                .cancel_download(&task_id).await.ok()
        },
        |_| Message::TaskRemoved(task_id)
    )
}
```

**Testing Required**:
- [ ] Unit test: pause/resume/cancel logic
- [ ] Integration test: pause during download
- [ ] UI test: buttons respond correctly

**Assigned To**: TBD  
**Status**: OPEN  
**Target Release**: v0.2.0

---

### 🟢 BUG-005: Excessive Clippy Warnings (Code Quality)
- **Priority**: P4 (LOW)
- **Severity**: LOW
- **Component**: All modules
- **Files**: Multiple
- **Count**: 59 warnings
- **Discovered**: 2025-11-23 (cargo clippy)

**Description**:
Codebase contains 59 clippy warnings across multiple categories:
- 17 unused imports
- 14 unused variables
- 11 dead code warnings
- 4 unused mut
- 13 other clippy warnings

**Impact**:
- Code readability reduced
- Potential hiding of real bugs
- Developer confusion
- Maintenance difficulty
- Poor first impression for contributors

**Examples**:
```rust
// Unused imports
use std::process::Command;  // Never used
use reqwest::Response;      // Never used

// Unused variables  
let entries = ...;  // Never read
let runtime = ...;  // Never read

// Unnecessary mut
let mut bridge = ...; // Never mutated
```

**Recommended Fix**:
```bash
# Step 1: Auto-fix what's safe
cargo clippy --fix --allow-dirty --allow-staged

# Step 2: Manual review remaining warnings
cargo clippy --all-targets --all-features

# Step 3: Suppress false positives
#[allow(clippy::large_enum_variant)]
pub enum ProgressUpdate { ... }
```

**Effort Estimate**:
- Auto-fix: 30 minutes
- Manual review: 2 hours
- Testing: 1 hour
- **Total**: 3-4 hours

**Assigned To**: TBD  
**Status**: OPEN  
**Target Release**: v0.2.0

---

## RESOLVED ISSUES

### ✅ BUG-000: Quality Tier Detection Test Failure
- **Priority**: P1 (CRITICAL)
- **Severity**: HIGH
- **Component**: File Organizer
- **File**: `src/utils/organizer.rs`
- **Line**: 266
- **Discovered**: 2025-11-23 (cargo test)
- **Resolved**: 2025-11-23

**Description**:
Test `test_quality_tier_detection` failed because "4K" quality string wasn't recognized as high quality. The regex-based extraction only looked for numeric values.

**Fix Applied**:
```rust
pub fn determine_quality_tier(quality: &str) -> QualityTier {
    let quality_lower = quality.to_lowercase();
    
    // Handle special cases FIRST
    if quality_lower.contains("4k") || quality_lower.contains("2160") {
        return QualityTier::HighQuality;
    }
    
    // Then numeric extraction
    let resolution = quality.chars()
        .filter(|c| c.is_numeric())
        .collect::<String>()
        .parse::<u32>()
        .unwrap_or(0);
    
    if resolution >= 1080 { QualityTier::HighQuality }
    else if resolution >= 480 { QualityTier::Standard }
    else { QualityTier::LowQuality }
}
```

**Verification**:
```
test utils::organizer::tests::test_quality_tier_detection ... ok
```

**Status**: CLOSED  
**Resolution Date**: 2025-11-23

---

## ISSUE STATISTICS

| Severity | Open | Closed | Total |
|----------|------|--------|-------|
| Critical (P1) | 1 | 1 | 2 |
| High (P2) | 2 | 0 | 2 |
| Medium (P3) | 1 | 0 | 1 |
| Low (P4) | 1 | 0 | 1 |
| **Total** | **5** | **1** | **6** |

---

## RELEASE BLOCKERS

### v0.1.0-beta
- [ ] BUG-001: Mutex deadlock fix
- [ ] BUG-002: Security vulnerability mitigation

### v0.1.0 (Stable)
- [ ] BUG-001: Mutex deadlock fix
- [ ] BUG-002: Security vulnerability mitigation  
- [ ] BUG-004: Pause/resume/cancel implementation

### v1.0.0 (Production)
- [ ] All above
- [ ] BUG-003: Memory optimization
- [ ] BUG-005: Code quality cleanup
- [ ] Test coverage >80%
- [ ] Performance benchmarks meet targets

---

## HOW TO REPORT BUGS

### Bug Report Template
```markdown
### Bug Title
[Clear, concise description]

### Environment
- OS: [macOS/Windows/Linux]
- Rust Version: [rustc --version]
- Rustloader Version: [v0.1.0]

### Steps to Reproduce
1. [Step 1]
2. [Step 2]
3. [Step 3]

### Expected Behavior
[What should happen]

### Actual Behavior
[What actually happened]

### Logs
[Paste relevant logs]

### Screenshots
[If applicable]
```

### Submission Channels
- GitHub Issues: [repository URL]
- Email: [support email]
- Discord: [community link]

---

**Maintained by**: Rustloader QA Team  
**Last Review**: November 23, 2025  
**Next Review**: December 1, 2025
