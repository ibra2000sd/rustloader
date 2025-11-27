# Rustloader v0.1.0 → v0.1.1 - Quick Fix Checklist

**Target**: Address all P1/P2 issues before beta release  
**Estimated Effort**: 6-8 hours  
**Deadline**: Before public beta announcement

---

## ✅ QUICK WINS (Do First)

### Fix 1: Disable MySQL Feature (10 minutes)
**Priority**: P2 | **Impact**: Eliminates security vulnerability

```bash
# Edit Cargo.toml
```

**Changes**:
```toml
# Before
[dependencies]
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "chrono"] }

# After  
[dependencies]
sqlx = { 
    version = "0.8", 
    features = ["sqlite", "runtime-tokio", "chrono"],
    default-features = false  # 👈 ADD THIS LINE
}
```

**Verification**:
```bash
cargo clean
cargo build --release
cargo audit
# Expected: "0 vulnerabilities found"
```

✅ **Completed**: [ ]

---

### Fix 2: Fix Mutex Deadlock (2 hours)
**Priority**: P1 | **Impact**: Prevents application freezes

**File**: `src/gui/app.rs`

**Location 1** (Line ~221):
```rust
// BEFORE
Message::ExtractUrl(url) => {
    match self.backend_bridge.lock().unwrap().as_ref() {
        Ok(mut bridge) => bridge.extract_video_info(&url_clone).await,
        Err(e) => Err(e.clone()),
    }
}

// AFTER
Message::ExtractUrl(url) => {
    let bridge_result = self.backend_bridge.lock().unwrap().as_ref().cloned();
    match bridge_result {
        Ok(bridge) => bridge.extract_video_info(&url_clone).await,
        Err(e) => Err(e.clone()),
    }
}
```

**Location 2** (Line ~302):
```rust
// BEFORE
Message::StartDownload { video_info, output_dir } => {
    match self.backend_bridge.lock().unwrap().as_ref() {
        Ok(mut bridge) => bridge.start_download(vi_for_call.clone(), out.clone(), None).await,
        Err(e) => Err(e.clone()),
    }
}

// AFTER
Message::StartDownload { video_info, output_dir } => {
    let bridge_result = self.backend_bridge.lock().unwrap().as_ref().cloned();
    match bridge_result {
        Ok(bridge) => bridge.start_download(vi_for_call.clone(), out.clone(), None).await,
        Err(e) => Err(e.clone()),
    }
}
```

**Verification**:
```bash
cargo clippy --all-targets -- -D clippy::await_holding_lock
# Expected: 0 warnings
```

✅ **Completed**: [ ]

---

## 🔧 MEDIUM PRIORITY (Do if Time Permits)

### Fix 3: Wire Up Pause/Cancel Buttons (4 hours)
**Priority**: P2 | **Impact**: Makes UI buttons functional

**File**: `src/gui/app.rs`

**Add these message handlers**:
```rust
Message::PauseTask(task_id) => {
    let backend = self.backend_bridge.clone();
    let task_id_clone = task_id.clone();
    
    Command::perform(
        async move {
            if let Ok(guard) = backend.lock() {
                if let Ok(bridge) = guard.as_ref() {
                    bridge.pause_download(&task_id_clone).await.ok();
                }
            }
        },
        |_| Message::RefreshTasks
    )
}

Message::ResumeTask(task_id) => {
    let backend = self.backend_bridge.clone();
    let task_id_clone = task_id.clone();
    
    Command::perform(
        async move {
            if let Ok(guard) = backend.lock() {
                if let Ok(bridge) = guard.as_ref() {
                    bridge.resume_download(&task_id_clone).await.ok();
                }
            }
        },
        |_| Message::RefreshTasks
    )
}

Message::CancelTask(task_id) => {
    let backend = self.backend_bridge.clone();
    let task_id_clone = task_id.clone();
    
    Command::perform(
        async move {
            if let Ok(guard) = backend.lock() {
                if let Ok(bridge) = guard.as_ref() {
                    bridge.cancel_download(&task_id_clone).await.ok();
                }
            }
        },
        |_| Message::TaskRemoved(task_id_clone)
    )
}

Message::RemoveCompletedTask(task_id) => {
    let backend = self.backend_bridge.clone();
    let task_id_clone = task_id.clone();
    
    Command::perform(
        async move {
            if let Ok(guard) = backend.lock() {
                if let Ok(bridge) = guard.as_ref() {
                    bridge.remove_task(&task_id_clone).await.ok();
                }
            }
        },
        |_| Message::TaskRemoved(task_id_clone)
    )
}
```

**Verification**:
1. Start a download
2. Click "Pause" → verify download stops
3. Click "Resume" → verify download continues
4. Click "Cancel" → verify task removed from queue

✅ **Completed**: [ ]

---

### Fix 4: Reduce Clippy Warnings (3 hours)
**Priority**: P4 | **Impact**: Improves code quality

**Automated Fixes**:
```bash
# Remove unused imports/variables
cargo clippy --fix --allow-dirty --allow-staged

# Check remaining warnings
cargo clippy --all-targets --all-features 2>&1 | grep "warning:" | wc -l
# Target: <10 warnings
```

**Manual Fixes Needed**:
1. Remove `std::process::Command` import from `ytdlp.rs` (line 8)
2. Prefix unused variables with `_` (e.g., `_entries`, `_runtime`)
3. Add `#[allow(dead_code)]` to intentionally unused methods
4. Fix `large_enum_variant` warning (see Fix 5)

✅ **Completed**: [ ]

---

### Fix 5: Box Large Enum Variant (1 hour)
**Priority**: P3 | **Impact**: Reduces memory usage

**File**: `src/gui/integration.rs`

```rust
// BEFORE (Line 29)
pub enum ProgressUpdate {
    ExtractionComplete(VideoInfo),
    // ...
}

// AFTER
pub enum ProgressUpdate {
    ExtractionComplete(Box<VideoInfo>),  // 👈 Box it
    // ...
}
```

**Update all usages**:
```rust
// Sending
ProgressUpdate::ExtractionComplete(Box::new(video_info))

// Receiving
match update {
    ProgressUpdate::ExtractionComplete(video_info) => {
        // Use *video_info or just video_info (auto-deref)
    }
}
```

**Files to update**:
- `src/gui/integration.rs` (construction sites)
- `src/gui/app.rs` (pattern matching)
- `src/queue/manager.rs` (if applicable)

**Verification**:
```bash
cargo clippy -- -D clippy::large_enum_variant
# Expected: 0 warnings
```

✅ **Completed**: [ ]

---

## 🧪 TESTING CHECKLIST

After implementing fixes, verify:

### Build & Compile
- [ ] `cargo clean && cargo build --release` succeeds
- [ ] No compilation errors
- [ ] Clippy warnings reduced to <10
- [ ] Binary size ~20MB (no significant increase)

### Security
- [ ] `cargo audit` shows 0 vulnerabilities
- [ ] No HIGH or CRITICAL findings

### Functional Tests
- [ ] All unit tests pass: `cargo test --release --lib`
- [ ] Application launches without crashes
- [ ] GUI renders correctly
- [ ] Video extraction works
- [ ] Download starts successfully
- [ ] Pause button works
- [ ] Resume button works
- [ ] Cancel button works
- [ ] Settings persist across restarts

### Manual QA
- [ ] Test with YouTube URL
- [ ] Test with Vimeo URL  
- [ ] Test with invalid URL (error handling)
- [ ] Test concurrent downloads (2-3 simultaneous)
- [ ] Test long-running download (>5 minutes)
- [ ] Verify no memory leaks (Activity Monitor)

---

## 📋 RELEASE STEPS (v0.1.1)

1. **Complete All Fixes**
   - [ ] Fix 1: Disable MySQL feature
   - [ ] Fix 2: Fix mutex deadlock
   - [ ] Fix 3: Wire up pause/cancel (optional)
   - [ ] Fix 4: Reduce clippy warnings (partial okay)
   - [ ] Fix 5: Box enum variant (optional)

2. **Update Version**
   ```toml
   # Cargo.toml
   [package]
   version = "0.1.1"
   ```

3. **Run Full Test Suite**
   ```bash
   cargo test --release --all
   cargo clippy --all-targets --all-features
   cargo audit
   ```

4. **Build Release Binary**
   ```bash
   cargo build --release
   strip target/release/rustloader  # Remove debug symbols
   ```

5. **Create Git Tag**
   ```bash
   git add .
   git commit -m "Release v0.1.1 - Security fixes and mutex deadlock resolution"
   git tag -a v0.1.1 -m "Version 0.1.1"
   git push origin main --tags
   ```

6. **Package for Distribution**
   ```bash
   # macOS
   tar -czf rustloader-v0.1.1-macos.tar.gz target/release/rustloader
   
   # Generate checksums
   shasum -a 256 rustloader-v0.1.1-macos.tar.gz > checksums.txt
   ```

7. **Update Documentation**
   - [ ] Update README.md with new version
   - [ ] Update CHANGELOG.md with fixes
   - [ ] Close resolved issues in tracker

8. **Announce Release**
   - [ ] GitHub Release with notes
   - [ ] Discord/community announcement
   - [ ] Update project website

---

## 🚨 ROLLBACK PLAN

If critical bugs found after release:

1. **Immediate**:
   ```bash
   git revert HEAD
   git push origin main
   ```

2. **Communication**:
   - Post hotfix notice on GitHub
   - Warn users not to upgrade
   - Provide rollback instructions

3. **Fix & Retest**:
   - Address critical bug
   - Run full test suite again
   - Release v0.1.2 as hotfix

---

## 📊 PROGRESS TRACKER

| Task | Priority | Status | ETA |
|------|----------|--------|-----|
| Fix 1: MySQL feature | P2 | ⬜ Not Started | 10m |
| Fix 2: Mutex deadlock | P1 | ⬜ Not Started | 2h |
| Fix 3: Pause/cancel UI | P2 | ⬜ Not Started | 4h |
| Fix 4: Clippy warnings | P4 | ⬜ Not Started | 3h |
| Fix 5: Enum boxing | P3 | ⬜ Not Started | 1h |
| Testing | - | ⬜ Not Started | 2h |
| Release | - | ⬜ Not Started | 1h |

**Total**: 13 hours estimated

---

**Last Updated**: November 23, 2025  
**Assignee**: TBD  
**Target Date**: [Set deadline]
