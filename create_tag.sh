#!/bin/bash
cd "/Users/hanafi/rustprojects/Rust_loader copy"
git tag -d v0.1.1-beta 2>/dev/null || true
git tag -a v0.1.1-beta -F tag_message.txt
git tag
echo "Tag created successfully!"
