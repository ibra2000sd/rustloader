# Rustloader - High-Performance Video Downloader

Rustloader is a cross-platform video downloader that combines the extraction capabilities of yt-dlp with a blazing-fast Rust-based download engine and a simple, practical GUI.

## Features

- **Multi-threaded downloading**: Up to 16 parallel segments for maximum speed
- **1000+ site support**: Powered by yt-dlp for broad compatibility
- **Resume capability**: Pause and resume downloads without data loss
- **Queue management**: Handle multiple downloads concurrently
- **Simple, practical GUI**: Clean interface focused on functionality

## Installation

### Prerequisites

1. Install Rust (latest stable)
2. Install yt-dlp:
   ```bash
   pip install yt-dlp
   ```
   or visit [yt-dlp releases](https://github.com/yt-dlp/yt-dlp/releases)

### Building from Source

```bash
git clone https://github.com/yourusername/rustloader.git
cd rustloader
cargo build --release
```

## Usage

### GUI Mode

Simply run the application:

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/rustloader
```

### Command Line Testing

Test a download without GUI:

```bash
cargo run --release -- --test-download "https://www.youtube.com/watch?v=VIDEO_ID"
```

## Configuration

The application settings can be configured through the Settings panel:

- **Download Location**: Where to save downloaded videos
- **Max Concurrent Downloads**: Number of simultaneous downloads (1-10)
- **Segments per Download**: Number of parallel segments (4-32)
- **Quality**: Preferred video quality

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                RUSTLOADER                       │
│                                                 │
│  ┌──────────────┐    ┌─────────────────────┐      │
│  │    GUI      │◄──►│   Core Logic      │      │
│  │   (Iced)    │    │      Layer        │      │
│  └──────────────┘    └─────────────────────┘      │
│                            │                   │
│  ┌────────────────────────┼────────────────┐    │
│  ▼                    ▼                 ▼    │
│  ┌─────────────┐  ┌────────────┐  ┌───────┐ │
│  │   yt-dlp   │  │ Download   │  │  DB   │ │
│  │ Extractor  │  │  Engine    │  │SQLite │ │
│  │ (Wrapper)  │  │  (Rust)    │  └───────┘ │
│  └─────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────┘
```

## Performance

Rustloader achieves 5-10x faster download speeds compared to vanilla yt-dlp by:

- Using multiple parallel connections
- Optimizing segment requests
- Implementing efficient merging algorithms
- Minimizing memory usage

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
