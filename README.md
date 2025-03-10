# Rustloader

<div align="center">

**Advanced Video Downloader built with Rust**

[![GitHub license](https://img.shields.io/github/license/ibra2000sd/rustloader)](https://github.com/ibra2000sd/rustloader/blob/main/LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/ibra2000sd/rustloader)](https://github.com/ibra2000sd/rustloader/stargazers)

</div>

Rustloader is a powerful, versatile command-line tool for downloading videos and audio from various online platforms. Built with Rust for maximum performance and reliability.

## Features

### Free Version
- Download videos up to 720p quality
- Extract MP3 audio at 128kbps
- Download specific segments using start and end time markers
- Download entire playlists
- Automatically fetch subtitles
- Progress bar tracking
- Desktop notifications when downloads complete
- Automatic dependency checking and updates

### Pro Version
- High quality video downloads (1080p, 4K, 8K)
- High-fidelity audio extraction (320kbps, FLAC)
- No daily download limits
- Multi-threaded downloads for maximum speed
- Priority updates and support
- Unlimited downloads

## Required Dependencies

Rustloader depends on these external tools:

- **yt-dlp** - For video extraction
- **ffmpeg** - For media processing

The automatic installation script will install these for you. If you're installing manually, see the Manual Dependencies Installation section below.

## Installation

### Method 1: Automatic Installation Script (Recommended)

For Linux and macOS users, we provide an automatic installation script that handles everything for you:

```bash
curl -sSL https://raw.githubusercontent.com/ibra2000sd/rustloader/main/install.sh | bash
```

This script will:
- Install Rust if not already installed
- Install all dependencies (yt-dlp and ffmpeg)
- Build and install rustloader
- Add rustloader to your system PATH
- Verify the installation works correctly

For security-conscious users, you can download the script first, review it, and then run it:

```bash
curl -O https://raw.githubusercontent.com/ibra2000sd/rustloader/main/install.sh
chmod +x install.sh
./install.sh
```

### Method 2: Install from Source

1. **Install Rust and Cargo** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/ibra2000sd/rustloader.git
   cd rustloader
   ```

3. **Build and install**:
   ```bash
   cargo install --path .
   ```

### Method 3: Manual Dependencies Installation

Rustloader will check for and notify you about missing dependencies, but you can install them ahead of time:

#### On macOS (using Homebrew):
```bash
brew install yt-dlp ffmpeg
```

#### On Linux (Debian/Ubuntu):
```bash
sudo apt update
sudo apt install python3 python3-pip ffmpeg
pip3 install --user --upgrade yt-dlp
```

#### On Windows:
1. Install yt-dlp: https://github.com/yt-dlp/yt-dlp#installation
2. Install ffmpeg: https://ffmpeg.org/download.html#build-windows

## Adding to System PATH

### Linux/macOS

If you've installed using `cargo install`, the binary is automatically added to your PATH at `~/.cargo/bin/rustloader`.

To manually add to PATH:

1. **Find the binary location**:
   ```bash
   which rustloader
   ```

2. **Add to your shell profile** (`.bashrc`, `.zshrc`, etc.):
   ```bash
   echo 'export PATH=$PATH:/path/to/rustloader/binary' >> ~/.bashrc
   source ~/.bashrc
   ```

### Windows

1. **Find the binary location** (typically in `%USERPROFILE%\.cargo\bin`)

2. **Add to PATH**:
   - Right-click on 'This PC' or 'My Computer' and select 'Properties'
   - Click on 'Advanced system settings'
   - Click the 'Environment Variables' button
   - Under 'System variables', find and select 'Path', then click 'Edit'
   - Click 'New' and add the path to the directory containing rustloader.exe
   - Click 'OK' on all dialogs to save changes

## Usage

### Basic Usage

```bash
rustloader [URL] [OPTIONS]
```

### Getting Help

To see all available options and commands:

```bash
rustloader --help
# or
rustloader -h
```

This displays a comprehensive help message with all available options, arguments, and their descriptions.

### Examples

1. **Download a video in default quality**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ
   ```

2. **Download in specific quality**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --quality 720
   ```

3. **Download audio only**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --format mp3
   ```

4. **Download a specific section**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --start-time 00:01:30 --end-time 00:02:45
   ```

5. **Download with subtitles**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --subs
   ```

6. **Download a playlist**:
   ```bash
   rustloader https://www.youtube.com/playlist?list=PLxxxxxxx --playlist
   ```

7. **Specify output directory**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --output-dir ~/Videos/music
   ```

### Pro Version Activation

If you have purchased a Pro license, you can activate it with:

```bash
rustloader --activate YOUR_LICENSE_KEY
```

To check your license status:

```bash
rustloader --license
```

### Available Options

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Display help information |
| `--version` | `-V` | Display version information |
| `--quality` | `-q` | Video quality (480, 720, 1080) |
| `--format` | `-f` | Output format (mp4, mp3) |
| `--start-time` | `-s` | Start time (HH:MM:SS) |
| `--end-time` | `-e` | End time (HH:MM:SS) |
| `--playlist` | `-p` | Download entire playlist |
| `--subs` | | Download subtitles if available |
| `--output-dir` | `-o` | Specify custom output directory |
| `--bitrate` | | Set video bitrate (e.g., 1000K) |
| `--activate` | | Activate a Pro license |
| `--license` | | Display license information |

## Troubleshooting

### Daily Download Limit

Free version users are limited to 5 downloads per day. To remove this limitation, consider upgrading to the Pro version.

### 403 Forbidden Errors

If you encounter a 403 Forbidden error, it might be because YouTube is detecting automated downloads.

Solutions:
1. Update yt-dlp to the latest version (Rustloader attempts this automatically)
2. Create a cookies.txt file in your home directory (~/.cookies.txt) by exporting cookies from your browser

### Other Issues

- Make sure both yt-dlp and ffmpeg are installed and in your PATH
- Check that you have write permissions for the download directory
- Verify that your internet connection is stable

## Security Features

Rustloader includes several security features:
- Secure license verification system
- Path validation to prevent directory traversal
- Input sanitization for command arguments
- Safe file operations
- Anti-tampering protections

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) for the video extraction capabilities
- [ffmpeg](https://ffmpeg.org/) for media processing

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
