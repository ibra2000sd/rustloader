# Rustloader

Rustloader is a command-line tool for downloading videos and audio clips. It supports specifying quality, format (MP4/MP3), and time 
ranges for clips. Built with Rust for speed and reliability, Rustloader integrates seamlessly with tools like `yt-dlp` and `ffmpeg` to 
provide a powerful downloading experience.

## Features

- **Video Downloads**: Download videos in MP4 format with specified quality (480p, 720p, 1080p).
- **Audio Extraction**: Extract audio as MP3 files.
- **Custom Time Ranges**: Download specific parts of a video or audio by providing start and end times.
- **Automatic Dependencies Handling**: Ensures `yt-dlp` and `ffmpeg` are installed and up to date.
- **Cross-Platform**: Works on macOS, Linux, and Windows.

## Prerequisites

Before using Rustloader, ensure the following tools are installed:

- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp)
- [`ffmpeg`](https://ffmpeg.org/)
- [Rust](https://www.rust-lang.org/)

For macOS users, these can be installed using [Homebrew](https://brew.sh/):
```bash
brew install yt-dlp ffmpeg
```

## Installation

### 1. Clone the Repository
```bash
git clone https://github.com/your-username/rustloader.git
cd rustloader
```

### 2. Build the Project
Build the project in release mode:
```bash
cargo build --release
```

### 3. Install the Binary
Move the compiled binary to a directory in your `PATH` (e.g., `/usr/local/bin`):
```bash
sudo mv target/release/rustloader /usr/local/bin/rustloader
```

### 4. Verify Installation
Run the following command to ensure Rustloader is accessible:
```bash
rustloader --help
```

## Usage

### Basic Command
To download a video:
```bash
rustloader <url>
```

### Options

- **`--quality` or `-q`**: Specify video quality (e.g., 480, 720, 1080).
- **`--format` or `-f`**: Specify the format (`mp4` or `mp3`).
- **`--start-time` or `-s`**: Start time of the clip (e.g., `00:01:00`).
- **`--end-time` or `-e`**: End time of the clip (e.g., `00:02:00`).

### Examples

1. **Download a video in 720p**:
   ```bash
   rustloader -q 720 <url>
   ```

2. **Download audio as MP3**:
   ```bash
   rustloader -f mp3 <url>
   ```

3. **Download a specific clip (from 1:00 to 2:00)**:
   ```bash
   rustloader -s 00:01:00 -e 00:02:00 <url>
   ```

4. **Download the best quality available**:
   ```bash
   rustloader <url>
   ```

## Development

### Run the Project
You can run the project directly using `cargo`:
```bash
cargo run -- <url>
```

### Run Tests
To run the tests:
```bash
cargo test
```

## Contributing

Contributions are welcome! If you encounter any bugs or have suggestions for new features, feel free to open an issue or submit a pull 
request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) for video downloading functionality.
- [ffmpeg](https://ffmpeg.org/) for video and audio processing.
- [Rust](https://www.rust-lang.org/) for making fast and reliable tools possible.
