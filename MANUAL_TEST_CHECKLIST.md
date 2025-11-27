# 🧪 Rustloader v0.1.1 - Manual Test Checklist

**Tester**: _______________  
**Date**: _______________  
**Platform**: macOS _____ (version)  
**Build**: v0.1.1 release

---

## 🎯 PRE-TEST SETUP

### Environment Verification
- [ ] yt-dlp installed and accessible (`yt-dlp --version`)
- [ ] Application launches without errors
- [ ] Download directory has >5GB free space
- [ ] Stable internet connection (>10Mbps)

### Test URLs (Copy these for testing)
```
# Short video (fast test)
https://www.youtube.com/watch?v=jNQXAC9IVRw

# Medium video (pause/resume test)
https://www.youtube.com/watch?v=dQw4w9WgXcQ

# Long video (stress test)
https://www.youtube.com/watch?v=9bZkp7q19f0

# Vimeo (multi-platform test)
https://vimeo.com/148751763
```

---

## TEST 1: No Freezing During Concurrent Extractions ⏱️ 5 MIN

**Purpose**: Verify BUG-001 fix (mutex deadlock)

### Steps:
1. [ ] Launch application: `cargo run --release`
2. [ ] Paste YouTube URL #1 and click "Extract Info"
3. [ ] **IMMEDIATELY** paste URL #2 and click "Extract Info"
4. [ ] **IMMEDIATELY** paste URL #3 and click "Extract Info"
5. [ ] Repeat with 2 more URLs (total: 5 extractions)

### Success Criteria:
- [ ] ✅ All 5 extractions complete
- [ ] ✅ UI remains responsive throughout
- [ ] ✅ No application freeze or hang
- [ ] ✅ All video info displays correctly

### Result: ⬜ PASS / ⬜ FAIL

**Notes**: _________________________

---

## TEST 2: Pause Functionality ⏱️ 5 MIN

**Purpose**: Verify pause button stops download

### Steps:
1. [ ] Start medium-length video download (>50MB)
2. [ ] Let progress reach approximately 25%
3. [ ] Note exact progress percentage: _____%
4. [ ] Click "Pause" button
5. [ ] Wait 10 seconds
6. [ ] Observe progress bar

### Success Criteria:
- [ ] ✅ Progress stops immediately after clicking pause
- [ ] ✅ Progress percentage doesn't change during 10-second wait
- [ ] ✅ Download speed shows 0 MB/s
- [ ] ✅ Status shows "Paused" or similar
- [ ] ✅ No error messages displayed

### Result: ⬜ PASS / ⬜ FAIL

**Progress when paused**: _____%  
**Notes**: _________________________

---

## TEST 3: Resume Functionality ⏱️ 5 MIN

**Purpose**: Verify resume continues from pause point (not restart)

### Steps:
1. [ ] Using paused download from TEST 2
2. [ ] Note progress before resume: _____%
3. [ ] Click "Resume" button
4. [ ] Observe progress bar for 30 seconds
5. [ ] Let download run to approximately 50%

### Success Criteria:
- [ ] ✅ Download continues from paused point (NOT from 0%)
- [ ] ✅ Progress increases smoothly
- [ ] ✅ Download speed shows MB/s value
- [ ] ✅ Status shows "Downloading" or similar
- [ ] ✅ No re-download of already completed segments

### Result: ⬜ PASS / ⬜ FAIL

**Progress after resume**: _____%  
**Did it restart from 0%?**: ⬜ Yes (FAIL) / ⬜ No (PASS)  
**Notes**: _________________________

---

## TEST 4: Cancel Functionality ⏱️ 3 MIN

**Purpose**: Verify cancel removes task completely

### Steps:
1. [ ] Start a new download
2. [ ] Let progress reach approximately 30%
3. [ ] Click "Cancel" button
4. [ ] Check download queue/list
5. [ ] Check downloads folder for partial files

### Success Criteria:
- [ ] ✅ Task immediately removed from queue
- [ ] ✅ Task no longer visible in UI
- [ ] ✅ Download stops immediately
- [ ] ✅ No error messages or crashes
- [ ] ⚠️ Partial file handling (either deleted or kept - both acceptable)

### Result: ⬜ PASS / ⬜ FAIL

**Partial file status**: ⬜ Deleted / ⬜ Kept  
**Notes**: _________________________

---

## TEST 5: Concurrent Multi-Task Operations ⏱️ 10 MIN

**Purpose**: Verify independent task control

### Steps:
1. [ ] Queue 3 different videos (label them A, B, C)
2. [ ] Start all 3 downloads simultaneously
3. [ ] Video A: Let reach ~20%, then PAUSE
4. [ ] Video B: Let reach ~40%, then CANCEL
5. [ ] Video C: Let complete to 100%
6. [ ] Video A: Click RESUME
7. [ ] Video A: Let complete to 100%

### Success Criteria:
- [ ] ✅ All 3 downloads start successfully
- [ ] ✅ Video A pauses at ~20% without affecting B or C
- [ ] ✅ Video B cancels at ~40% without affecting A or C
- [ ] ✅ Video C completes without interruption
- [ ] ✅ Video A resumes from ~20% (not 0%)
- [ ] ✅ Video A completes successfully
- [ ] ✅ Final result: 2 complete files (A and C)

### Result: ⬜ PASS / ⬜ FAIL

**Video A**: ⬜ Success / ⬜ Failed  
**Video B**: ⬜ Cancelled correctly / ⬜ Failed  
**Video C**: ⬜ Success / ⬜ Failed  
**Notes**: _________________________

---

## TEST 6: Edge Case - Rapid Button Clicks ⏱️ 3 MIN

**Purpose**: Verify UI handles rapid state changes

### Steps:
1. [ ] Start a download
2. [ ] Rapidly click: Pause → Resume → Pause → Resume → Cancel
   (Click each button within 1 second)
3. [ ] Observe application behavior

### Success Criteria:
- [ ] ✅ No application crash
- [ ] ✅ No error popups or exceptions
- [ ] ✅ Final state is consistent (task cancelled)
- [ ] ✅ UI remains responsive
- [ ] ✅ No zombie tasks or hung processes

### Result: ⬜ PASS / ⬜ FAIL

**Notes**: _________________________

---

## TEST 7: Remove Completed Tasks ⏱️ 2 MIN

**Purpose**: Verify cleanup functionality

### Steps:
1. [ ] Complete a download to 100%
2. [ ] Click "Remove" button (or similar cleanup button)
3. [ ] Observe task list

### Success Criteria:
- [ ] ✅ Completed task removed from list
- [ ] ✅ Downloaded file still exists in folder
- [ ] ✅ No errors or warnings

### Result: ⬜ PASS / ⬜ FAIL

**Notes**: _________________________

---

## TEST 8: Settings Persistence ⏱️ 3 MIN

**Purpose**: Verify settings save/load

### Steps:
1. [ ] Open Settings
2. [ ] Change download directory
3. [ ] Change max concurrent downloads
4. [ ] Save settings
5. [ ] Quit application
6. [ ] Relaunch application
7. [ ] Check Settings

### Success Criteria:
- [ ] ✅ Download directory persists
- [ ] ✅ Max concurrent setting persists
- [ ] ✅ All settings retained after restart

### Result: ⬜ PASS / ⬜ FAIL

**Notes**: _________________________

---

## TEST 9: Error Handling ⏱️ 5 MIN

**Purpose**: Verify graceful failure

### Steps:
1. [ ] Test invalid URL: `https://invalid-site.com/video`
2. [ ] Test deleted video (use known-deleted YouTube URL)
3. [ ] Test non-video URL: `https://www.google.com`
4. [ ] Test malformed URL: `not-a-url`

### Success Criteria:
- [ ] ✅ Clear error message for each case
- [ ] ✅ No application crashes
- [ ] ✅ Can continue using app after errors
- [ ] ✅ Error messages are user-friendly

### Result: ⬜ PASS / ⬜ FAIL

**Notes**: _________________________

---

## 📊 TEST SUMMARY

### Overall Results
- **Tests Passed**: _____ / 9
- **Tests Failed**: _____ / 9
- **Pass Rate**: _____% 

### Critical Tests (Must Pass)
- [ ] TEST 1: No Freezing (BUG-001 verification)
- [ ] TEST 2: Pause Works
- [ ] TEST 3: Resume Works
- [ ] TEST 4: Cancel Works

### Test Verdict
⬜ **APPROVED FOR BETA** - All critical tests passed  
⬜ **NEEDS FIXES** - One or more critical tests failed  
⬜ **BLOCKED** - Major issues found

---

## 🐛 ISSUES FOUND

Use this section to document any bugs discovered:

### Issue #1
**Severity**: ⬜ Critical / ⬜ High / ⬜ Medium / ⬜ Low  
**Test**: TEST _____  
**Description**: ___________________________  
**Steps to Reproduce**: ___________________________  
**Expected**: ___________________________  
**Actual**: ___________________________  

### Issue #2
**Severity**: ⬜ Critical / ⬜ High / ⬜ Medium / ⬜ Low  
**Test**: TEST _____  
**Description**: ___________________________  

---

## ✅ SIGN-OFF

**Tester Signature**: _______________  
**Date**: _______________  
**Recommendation**: _______________  

---

**Next Steps**:
- If all tests pass → Proceed with beta release
- If critical failures → Fix issues and re-test
- Report issues to: [GitHub Issues Link]
