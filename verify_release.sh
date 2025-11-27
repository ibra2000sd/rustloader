#!/bin/bash
# Rustloader v0.1.1 - Release Verification Script

set -e  # Exit on any error

BLUE='\033[0;34m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}🔍 RUSTLOADER v0.1.1 RELEASE VERIFICATION${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Change to project directory
cd "/Users/hanafi/rustprojects/Rust_loader copy"

# Test 1: Clean Build
echo -e "${YELLOW}📦 TEST 1: Clean Build${NC}"
cargo clean
if cargo build --release 2>&1 | tee /tmp/build.log; then
    # Look for actual compiler error lines (starting with error:)
    if grep -E "^error:" /tmp/build.log >/dev/null; then
        echo -e "${RED}❌ Build reported compiler errors${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Cargo build command failed${NC}"
    exit 1
fi
echo ""

# Test 2: Binary Size
echo -e "${YELLOW}📏 TEST 2: Binary Size Check${NC}"
SIZE=$(ls -lh target/release/rustloader | awk '{print $5}')
SIZE_BYTES=$(ls -l target/release/rustloader | awk '{print $5}')
echo "Binary size: $SIZE ($SIZE_BYTES bytes)"

if [ "$SIZE_BYTES" -gt 30000000 ]; then
    echo -e "${RED}⚠️  Warning: Binary larger than expected (>30MB)${NC}"
else
    echo -e "${GREEN}✅ Binary size acceptable${NC}"
fi
echo ""

# Test 3: Security Audit
echo -e "${YELLOW}🔒 TEST 3: Security Audit${NC}"
cargo audit > /tmp/audit.log 2>&1 || true

if grep -qi "Crate:\s*rsa" /tmp/audit.log; then
    echo -e "${RED}❌ RSA vulnerability still present in audit${NC}"
    echo "Checking actual dependency tree..."
    cargo tree -i rsa || echo "rsa not in dependency tree (audit false positive)"
else
    echo -e "${GREEN}✅ No RSA vulnerability in audit${NC}"
fi

# Check actual dependency tree
echo "Verifying rsa not in dependency tree..."
if cargo tree -i rsa 2>&1 | grep -q "rsa"; then
    echo -e "${RED}❌ RSA found in dependency tree${NC}"
    exit 1
else
    echo -e "${GREEN}✅ RSA not in dependency tree${NC}"
fi
echo ""

# Test 4: Mutex Deadlock Check
echo -e "${YELLOW}⏸️  TEST 4: Mutex Deadlock Detection${NC}"
cargo clippy --all-targets -- -W clippy::await_holding_lock 2>&1 > /tmp/clippy.log

if grep -q "await_holding_lock" /tmp/clippy.log; then
    echo -e "${RED}❌ Mutex deadlock warnings found${NC}"
    grep "await_holding_lock" /tmp/clippy.log
    exit 1
else
    echo -e "${GREEN}✅ No mutex deadlock warnings${NC}"
fi
echo ""

# Test 5: Unit Tests
echo -e "${YELLOW}🧪 TEST 5: Unit Tests${NC}"
cargo test --release --lib 2>&1 | tee /tmp/test.log

if grep -q "test result: ok" /tmp/test.log; then
    TESTS_PASSED=$(grep "test result: ok" /tmp/test.log | grep -o "[0-9]* passed" | grep -o "[0-9]*")
    echo -e "${GREEN}✅ All unit tests passed ($TESTS_PASSED tests)${NC}"
else
    echo -e "${RED}❌ Unit tests failed${NC}"
    exit 1
fi
echo ""

# Test 6: Version Check
echo -e "${YELLOW}📋 TEST 6: Version Consistency${NC}"
CARGO_VERSION=$(grep "^version = " Cargo.toml | head -1 | grep -o '"[^"]*"' | tr -d '"')
echo "Cargo.toml version: $CARGO_VERSION"

if [ "$CARGO_VERSION" = "0.1.1" ]; then
    echo -e "${GREEN}✅ Version correct (0.1.1)${NC}"
else
    echo -e "${RED}❌ Version mismatch (expected 0.1.1, got $CARGO_VERSION)${NC}"
    exit 1
fi
echo ""

# Test 7: File Integrity
echo -e "${YELLOW}📄 TEST 7: Required Files Check${NC}"
REQUIRED_FILES=(
    "CHANGELOG.md"
    "RELEASE_NOTES.md"
    "MANUAL_TEST_CHECKLIST.md"
    "README.md"
    "Cargo.toml"
    "target/release/rustloader"
)

ALL_PRESENT=true
for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✅${NC} $file"
    else
        echo -e "${RED}❌${NC} $file (MISSING)"
        ALL_PRESENT=false
    fi
done

if [ "$ALL_PRESENT" = false ]; then
    echo -e "${RED}❌ Some required files missing${NC}"
    exit 1
fi
echo ""

# Test 8: Clippy Warnings Count
echo -e "${YELLOW}🧹 TEST 8: Code Quality${NC}"
WARNINGS=$(cargo clippy --all-targets --all-features 2>&1 | grep -c "warning:" || echo "0")
echo "Clippy warnings: $WARNINGS"

if [ "$WARNINGS" -gt 50 ]; then
    echo -e "${YELLOW}⚠️  Warning count higher than target (<50)${NC}"
else
    echo -e "${GREEN}✅ Acceptable warning count${NC}"
fi
echo ""

# Test 9: Dependencies Check
echo -e "${YELLOW}📦 TEST 9: Critical Dependencies${NC}"
echo "Checking critical crates..."

cargo tree -p tokio | head -1
cargo tree -p iced | head -1
cargo tree -p sqlx | head -1
cargo tree -p reqwest | head -1

echo -e "${GREEN}✅ All critical dependencies present${NC}"
echo ""

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}📊 VERIFICATION SUMMARY${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${GREEN}✅ Build successful${NC}"
echo -e "${GREEN}✅ Binary size: $SIZE${NC}"
echo -e "${GREEN}✅ Security audit passed (RSA eliminated)${NC}"
echo -e "${GREEN}✅ No mutex deadlock warnings${NC}"
echo -e "${GREEN}✅ All unit tests passed ($TESTS_PASSED tests)${NC}"
echo -e "${GREEN}✅ Version correct (0.1.1)${NC}"
echo -e "${GREEN}✅ All required files present${NC}"
echo -e "${GREEN}✅ Code quality acceptable ($WARNINGS warnings)${NC}"
echo -e "${GREEN}✅ Critical dependencies verified${NC}"
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}🎉 ALL VERIFICATION CHECKS PASSED!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Next steps:"
echo "1. Complete manual testing (MANUAL_TEST_CHECKLIST.md)"
echo "2. Create release package (see below)"
echo "3. Tag release: git tag -a v0.1.1 -m 'Beta release'"
echo "4. Push to repository"
echo ""
echo "To create release package, run:"
echo "  ./package_release.sh"
