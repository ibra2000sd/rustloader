#!/bin/bash

# Create the icons directory
mkdir -p src-tauri/icons

# Create a simple placeholder icon using base64 to PNG
# This is a blue square icon - very simple for testing purposes
echo "iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAMAAAD04JH5AAAABlBMVEUAif8AAABi4KPkAAAAAXRSTlMAQObYZgAAAGJJREFUeNrt1bEJACAMBVEHcf9tdfRARAgZwHcDFDS80rHbGGdYwQICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAvKLJPZ7TvYbVPucAAAAAElFTkSuQmCC" | base64 -d > src-tauri/icons/icon.png

echo "Created icon file at src-tauri/icons/icon.png"

# Verify the file was created
if [ -f "src-tauri/icons/icon.png" ]; then
  echo "✅ Icon file created successfully"
  ls -la src-tauri/icons/
else
  echo "❌ Failed to create icon file"
fi