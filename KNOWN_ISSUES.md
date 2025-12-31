# Known Issues - Rustloader v0.1.1

This document tracks known issues and limitations in the current release.

---

## Current Issues

### 🟡 Medium Priority

#### ISSUE-001: Compiler Warnings
**Status**: Improved (reduced from 82 to ~15-20)
**Impact**: Development only (no user impact)
**Description**: Codebase has some compiler warnings, mostly related to unused code kept for future features.
**Workaround**: None needed - application functions correctly.
**Target Fix**: v0.1.2

#### ISSUE-002: Limited Automated Tests
**Status**: Planned
**Impact**: Development only
**Description**: Unit test coverage is limited, increasing regression risk.
**Target Fix**: v0.1.2

#### ISSUE-003: macOS Only
**Status**: Known Limitation
**Impact**: Users on Windows/Linux cannot use the application.
**Description**: Currently only tested and supported on macOS.
**Target Fix**: v0.2.0

### 🟢 Low Priority

#### ISSUE-004: Large Binary Size
**Status**: Accepted
**Impact**: Minor (longer download time)
**Description**: Release binary is ~90 MB due to GUI framework dependencies.
**Workaround**: None - this is expected for Iced-based applications.

#### ISSUE-005: Unmaintained Transitive Dependencies
**Status**: Monitoring
**Impact**: None currently
**Description**: Two transitive dependencies (instant, paste) are unmaintained.
**Note**: These come from the Iced framework and will be updated when Iced releases updates.

---

## Resolved Issues (v0.1.1)

| Issue | Description | Resolution |
|-------|-------------|------------|
| BUG-001 | Mutex deadlock in video extraction | Fixed - lock released before await |
| BUG-004 | Pause/Resume/Cancel non-functional | Fixed - proper state management |
| BUG-006 | Progress bars empty for subsequent downloads | Fixed - improved tracking |
| BUG-007 | Files not organized into directories | Fixed - directory validation |
| BUG-008 | Pause buttons disappear | Fixed - state string handling |
| SEC-001 | Path traversal in filenames | Fixed - comprehensive sanitization |

---

## Reporting New Issues

If you encounter a bug not listed here:

1. **Search existing issues**: [GitHub Issues](https://github.com/ibra2000sd/rustloader/issues)
2. **Create a new issue** with:
   - Steps to reproduce
   - Expected vs actual behavior
   - Your macOS version
   - Rustloader version (v0.1.1)
   - Any error messages or logs

---

**Last Updated**: December 31, 2025  
**Document Version**: 1.0
