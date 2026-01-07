param(
    [string]$Version = "0.8.0"
)

$DistDir = "dist\v$Version"

Write-Host "🔨 Building Rustloader v$Version for Windows..." -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# Build Windows x86_64
Write-Host "📦 Building Windows x86_64..." -ForegroundColor Yellow
cargo build --release --target x86_64-pc-windows-msvc

# Create zip
$BinaryPath = "target\x86_64-pc-windows-msvc\release\rustloader.exe"
$ZipPath = "$DistDir\rustloader-v$Version-windows-x86_64.zip"
Compress-Archive -Path $BinaryPath -DestinationPath $ZipPath -Force

# Generate checksum
Write-Host "🔐 Generating checksum..." -ForegroundColor Yellow
$Hash = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash
"$Hash  rustloader-v$Version-windows-x86_64.zip" | Out-File "$DistDir\SHA256SUMS.txt"

Write-Host "✅ Build complete! Artifacts in $DistDir" -ForegroundColor Green
