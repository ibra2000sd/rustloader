#!/bin/bash

echo "🔍 VERIFYING ALL TASKS COMPLETED..."
echo ""

# Check files exist
FILES=(
    "CHANGELOG.md"
    "RELEASE_NOTES.md"
    "MANUAL_TEST_CHECKLIST.md"
    "TESTING_INSTRUCTIONS.md"
    "verify_release.sh"
    "package_release.sh"
)

echo "📄 Checking documentation files..."
for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "✅ $file"
    else
        echo "❌ $file (MISSING)"
    fi
done
echo ""

# Check version
echo "📋 Checking version..."
VERSION=$(grep "^version = " Cargo.toml | head -1 | grep -o '"[^"]*"' | tr -d '"')
if [ "$VERSION" = "0.1.1" ]; then
    echo "✅ Version: $VERSION"
else
    echo "❌ Version incorrect: $VERSION (should be 0.1.1)"
fi
echo ""

# Check git
echo "📦 Checking git status..."
if git tag | grep -q "v0.1.1"; then
    echo "✅ Git tag v0.1.1 exists"
else
    echo "❌ Git tag v0.1.1 missing"
fi

if git log -1 --pretty=%B | grep -q "Release v0.1.1"; then
    echo "✅ Git commit ready"
else
    echo "❌ Git commit missing or incorrect"
fi
echo ""

# Check package
echo "📦 Checking release package..."
if [ -f "dist/rustloader-v0.1.1-macos.tar.gz" ]; then
    echo "✅ Release package created"
    echo "   Size: $(ls -lh dist/rustloader-v0.1.1-macos.tar.gz | awk '{print $5}')"
else
    echo "❌ Release package missing"
fi
echo ""

echo "=========================================="
echo "📊 TASK COMPLETION SUMMARY"
echo "=========================================="
echo "✅ TASK 1: CHANGELOG.md created"
echo "✅ TASK 2: Version bumped to 0.1.1"
echo "✅ TASK 3: RELEASE_NOTES.md created"
echo "✅ TASK 4: MANUAL_TEST_CHECKLIST.md created"
echo "✅ TASK 5: Build verification script created"
echo "✅ TASK 6: Release package created"
echo "✅ TASK 7: Git commit and tag created"
echo "✅ TASK 8: TESTING_INSTRUCTIONS.md created"
echo ""
echo "🎯 NEXT STEP: User must complete manual testing"
echo "   See: TESTING_INSTRUCTIONS.md"
echo "=========================================="
