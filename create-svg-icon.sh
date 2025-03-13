#!/bin/bash

# Create the icons directory
mkdir -p src-tauri/icons

# Create a simple SVG icon - these are text-based and very reliable
cat > "src-tauri/icons/icon.svg" << 'EOF'
<svg width="128" height="128" xmlns="http://www.w3.org/2000/svg">
  <rect width="128" height="128" fill="#3498db" rx="15" ry="15"/>
  <text x="50%" y="50%" dominant-baseline="middle" text-anchor="middle" font-family="Arial" font-size="80" fill="white">R</text>
</svg>
EOF

echo "Created SVG icon at src-tauri/icons/icon.svg"

# Attempt to convert SVG to PNG if Inkscape or rsvg-convert is available
if command -v rsvg-convert &> /dev/null; then
    echo "Converting SVG to PNG using rsvg-convert..."
    rsvg-convert -w 128 -h 128 src-tauri/icons/icon.svg > src-tauri/icons/icon.png
    echo "✅ Converted SVG to PNG using rsvg-convert"
elif command -v inkscape &> /dev/null; then
    echo "Converting SVG to PNG using Inkscape..."
    inkscape -z -w 128 -h 128 src-tauri/icons/icon.svg -e src-tauri/icons/icon.png
    echo "✅ Converted SVG to PNG using Inkscape"
else
    # If neither converter is available, use a basic PNG
    echo "No SVG to PNG converter found. Using a basic PNG instead."
    # The smallest valid PNG file
    echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==" | base64 --decode > src-tauri/icons/icon.png
    echo "✅ Created a basic PNG file"
fi

# Modify the tauri.conf.json to use SVG as a fallback if PNG conversion failed
if [ ! -f "src-tauri/icons/icon.png" ] && [ -f "src-tauri/icons/icon.svg" ]; then
    echo "PNG conversion failed. Updating tauri.conf.json to use SVG icon..."
    if [ -f "src-tauri/tauri.conf.json" ]; then
        sed -i 's/"icon": \[\]/"icon": ["icons\/icon.svg"]/' src-tauri/tauri.conf.json
        echo "✅ Updated tauri.conf.json to use SVG icon"
    fi
fi

# Final check
if [ -f "src-tauri/icons/icon.png" ]; then
    echo "✅ Final check: PNG icon file exists"
    ls -la src-tauri/icons/
    # Print file type information if file command is available
    if command -v file &> /dev/null; then
        file src-tauri/icons/icon.png
    fi
elif [ -f "src-tauri/icons/icon.svg" ]; then
    echo "✅ Final check: SVG icon file exists (PNG conversion failed)"
    ls -la src-tauri/icons/
else
    echo "❌ All attempts failed to create a valid icon file"
fi