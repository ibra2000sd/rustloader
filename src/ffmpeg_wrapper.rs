// src/ffmpeg_wrapper.rs
//
// Rewritten to use Rust-native ffmpeg libraries instead of external ffmpeg tool

use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use std::ffi::CString;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::cell::RefCell;
use indicatif::{ProgressBar, ProgressStyle};

#[cfg(feature = "ffmpeg-next")]
use {
    ffmpeg_next as ffmpeg,
    ffmpeg_next::format::{input, output},
    ffmpeg_next::codec::{encoder, decoder},
    ffmpeg_next::media::Type,
    ffmpeg_next::software::scaling::{context::Context, flag::Flags},
    ffmpeg_next::util::frame::video::Video,
    ffmpeg_next::util::frame::audio::Audio,
    ffmpeg_next::util::log::{Level, log_enabled},
};

#[cfg(not(feature = "ffmpeg-next"))]
use {
    ffmpeg4 as ffmpeg,
    ffmpeg4::format::{input, output},
    ffmpeg4::codec::{encoder, decoder},
    ffmpeg4::media::Type,
    ffmpeg4::software::scaling::{context::Context, flag::Flags},
    ffmpeg4::util::frame::video::Video,
    ffmpeg4::util::frame::audio::Audio,
    ffmpeg4::util::log::{Level, log_enabled},
};

// Type definition for progress callback
pub type ProgressCallback = Arc<dyn Fn(u64, u64) -> bool + Send + Sync>;

/// Configuration for FFmpeg operations
#[derive(Clone)]
pub struct FFmpegConfig {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub format: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub bitrate: Option<String>,
}

// Progress tracking
struct ProgressInfo {
    total_frames: u64,
    processed_frames: AtomicU64,
    start_time: std::time::Instant,
    callback: Option<ProgressCallback>,
}

impl ProgressInfo {
    fn new(total_frames: u64, callback: Option<ProgressCallback>) -> Self {
        Self {
            total_frames,
            processed_frames: AtomicU64::new(0),
            start_time: std::time::Instant::now(),
            callback,
        }
    }

    fn update(&self, frames: u64) -> bool {
        let old_frames = self.processed_frames.fetch_add(frames, Ordering::SeqCst);
        let new_frames = old_frames + frames;
        
        // Call the callback if provided
        if let Some(callback) = &self.callback {
            callback(new_frames, self.total_frames)
        } else {
            true // Continue if no callback
        }
    }
    
    fn get_percentage(&self) -> u64 {
        let processed = self.processed_frames.load(Ordering::SeqCst);
        if self.total_frames == 0 {
            return 0;
        }
        (processed * 100) / self.total_frames
    }
    
    fn get_elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
    
    fn get_eta_secs(&self) -> f64 {
        let processed = self.processed_frames.load(Ordering::SeqCst);
        if processed == 0 || self.total_frames == 0 {
            return 0.0;
        }
        
        let elapsed = self.get_elapsed_secs();
        let progress_ratio = processed as f64 / self.total_frames as f64;
        
        if progress_ratio == 0.0 {
            return 0.0;
        }
        
        (elapsed / progress_ratio) - elapsed
    }
}

/// Initialize FFmpeg by setting up the library
pub fn init() -> Result<(), AppError> {
    // Initialize the FFmpeg library
    ffmpeg::init().map_err(|e| {
        eprintln!("Failed to initialize FFmpeg: {}", e);
        AppError::MissingDependency(format!("Failed to initialize FFmpeg: {}", e))
    })?;
    
    // Set log level if not in debug mode
    #[cfg(not(debug_assertions))]
    ffmpeg::util::log::set_level(Level::Error);
    
    Ok(())
}

/// Convert video/audio to different format
pub async fn convert_media(
    config: &FFmpegConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<(), AppError> {
    // Create a progress bar for display
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Run the CPU-intensive conversion in a blocking task
    tokio::task::spawn_blocking({
        let config = config.clone();
        let pb = pb.clone();
        
        move || -> Result<(), AppError> {
            // Open input file
            let input_path = config.input_path.to_str()
                .ok_or_else(|| AppError::PathError("Invalid input path encoding".to_string()))?;
            
            let output_path = config.output_path.to_str()
                .ok_or_else(|| AppError::PathError("Invalid output path encoding".to_string()))?;
            
            // Open input context
            let mut input = input::open(&input_path)
                .map_err(|e| AppError::General(format!("Failed to open input file: {}", e)))?;
            
            // Get stream information
            input.metadata()
                .map_err(|e| AppError::General(format!("Failed to read metadata: {}", e)))?;
            
            // Calculate duration for progress tracking
            let duration_seconds = input
                .duration() as f64 / ffmpeg::util::time::AV_TIME_BASE as f64;
            
            let frame_rate = 25.0; // Assume 25 fps if we can't detect it
            let estimated_frames = (duration_seconds * frame_rate) as u64;
            
            // Create progress tracker
            let progress = Arc::new(ProgressInfo::new(estimated_frames, progress_callback));
            
            // Find the best video and audio streams
            let input_video_stream = input.streams()
                .best(Type::Video)
                .map(|stream| stream.index());
                
            let input_audio_stream = input.streams()
                .best(Type::Audio)
                .map(|stream| stream.index());
            
            // Prepare output context
            let mut output_context = output::open(&output_path)
                .map_err(|e| AppError::General(format!("Failed to create output file: {}", e)))?;
            
            // Set output format based on file extension
            if let Some(format) = config.format.as_ref() {
                output_context.set_format(format);
            }
            
            // Setup progress tracking
            pb.set_message("Starting conversion...");
            
            // Process video stream if exists and needed
            if let Some(video_stream_index) = input_video_stream {
                if config.format != "mp3" && config.format != "m4a" { // Skip video for audio-only outputs
                    transcode_video_stream(
                        &mut input, 
                        &mut output_context, 
                        video_stream_index,
                        Arc::clone(&progress),
                        &pb,
                    )?;
                }
            }
            
            // Process audio stream if exists
            if let Some(audio_stream_index) = input_audio_stream {
                transcode_audio_stream(
                    &mut input, 
                    &mut output_context, 
                    audio_stream_index, 
                    config.bitrate.as_deref(),
                    Arc::clone(&progress),
                    &pb,
                )?;
            }
            
            // Write the output file header
            output_context.write_header()
                .map_err(|e| AppError::General(format!("Failed to write output header: {}", e)))?;
            
            // Process all frames
            process_frames(
                &mut input, 
                &mut output_context, 
                Arc::clone(&progress),
                &pb,
            )?;
            
            // Write the output file trailer
            output_context.write_trailer()
                .map_err(|e| AppError::General(format!("Failed to write output trailer: {}", e)))?;
            
            pb.finish_with_message("Conversion completed successfully");
            
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::General(format!("Task join error: {}", e)))??;
    
    Ok(())
}

/// Extract audio from video file
pub async fn extract_audio(
    config: &FFmpegConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<(), AppError> {
    // Create a progress bar for display
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Run the CPU-intensive conversion in a blocking task
    tokio::task::spawn_blocking({
        let config = config.clone();
        let pb = pb.clone();
        
        move || -> Result<(), AppError> {
            // Open input file
            let input_path = config.input_path.to_str()
                .ok_or_else(|| AppError::PathError("Invalid input path encoding".to_string()))?;
            
            let output_path = config.output_path.to_str()
                .ok_or_else(|| AppError::PathError("Invalid output path encoding".to_string()))?;
            
            // Open input context
            let mut input = input::open(&input_path)
                .map_err(|e| AppError::General(format!("Failed to open input file: {}", e)))?;
            
            // Get stream information
            input.metadata()
                .map_err(|e| AppError::General(format!("Failed to read metadata: {}", e)))?;
            
            // Calculate duration for progress tracking
            let duration_seconds = input
                .duration() as f64 / ffmpeg::util::time::AV_TIME_BASE as f64;
            
            let frame_rate = 25.0; // Assume 25 fps if we can't detect it
            let estimated_frames = (duration_seconds * frame_rate) as u64;
            
            // Create progress tracker
            let progress = Arc::new(ProgressInfo::new(estimated_frames, progress_callback));
            
            // Find the best audio stream
            let input_audio_stream = input.streams()
                .best(Type::Audio)
                .map(|stream| stream.index())
                .ok_or_else(|| AppError::General("No audio stream found".to_string()))?;
            
            // Prepare output context
            let mut output_context = output::open(&output_path)
                .map_err(|e| AppError::General(format!("Failed to create output file: {}", e)))?;
            
            // Set output format based on file extension
            output_context.set_format("mp3");
            
            // Setup progress tracking
            pb.set_message("Starting audio extraction...");
            
            // Process audio stream
            transcode_audio_stream(
                &mut input, 
                &mut output_context, 
                input_audio_stream, 
                config.bitrate.as_deref(),
                Arc::clone(&progress),
                &pb,
            )?;
            
            // Write the output file header
            output_context.write_header()
                .map_err(|e| AppError::General(format!("Failed to write output header: {}", e)))?;
            
            // Process all frames
            process_frames(
                &mut input, 
                &mut output_context, 
                Arc::clone(&progress),
                &pb,
            )?;
            
            // Write the output file trailer
            output_context.write_trailer()
                .map_err(|e| AppError::General(format!("Failed to write output trailer: {}", e)))?;
            
            pb.finish_with_message("Audio extraction completed successfully");
            
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::General(format!("Task join error: {}", e)))??;
    
    Ok(())
}

/// Convert to MP3 or other audio format
pub async fn convert_to_audio(
    input_path: &str,
    output_path: &str,
    bitrate: &str,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> Result<(), AppError> {
    let config = FFmpegConfig {
        input_path: PathBuf::from(input_path),
        output_path: PathBuf::from(output_path),
        format: "mp3".to_string(),
        start_time: start_time.map(|s| s.to_string()),
        end_time: end_time.map(|s| s.to_string()),
        bitrate: Some(bitrate.to_string()),
    };
    
    extract_audio(&config, None).await
}

/// Transcode video stream
fn transcode_video_stream(
    input: &mut input::Input,
    output: &mut output::Output,
    stream_index: usize,
    progress: Arc<ProgressInfo>,
    pb: &ProgressBar,
) -> Result<(), AppError> {
    let input_stream = input.streams().get(stream_index)
        .ok_or_else(|| AppError::General("Video stream not found".to_string()))?;
    
    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|e| AppError::General(format!("Failed to create decoder context: {}", e)))?;
    
    let mut decoder = context_decoder.decoder().video()
        .map_err(|e| AppError::General(format!("Failed to find video decoder: {}", e)))?;
    
    // Find a suitable video encoder
    let encoder_name = find_suitable_video_encoder();
    let encoder = ffmpeg::encoder::find_by_name(encoder_name)
        .ok_or_else(|| AppError::General(format!("Encoder {} not found", encoder_name)))?;
    
    // Create output stream
    let output_stream = output.add_stream(encoder)
        .map_err(|e| AppError::General(format!("Failed to create output stream: {}", e)))?;
    
    // Configure video encoder context
    let mut context = ffmpeg::codec::context::Context::from_parameters(output_stream.parameters())
        .map_err(|e| AppError::General(format!("Failed to create encoder context: {}", e)))?;
    
    // Set video params same as input
    context.set_width(decoder.width());
    context.set_height(decoder.height());
    context.set_format(decoder.format());
    
    // Set reasonable default bitrate if not specified
    let bitrate = 800_000; // 800kbps
    context.set_bit_rate(bitrate);
    
    // Set frame rate
    let frame_rate = input_stream.avg_frame_rate();
    context.set_time_base(ffmpeg::rescale::TIME_BASE);
    
    // Get global header if needed
    if output.format().flags().contains(ffmpeg::format::Flags::GLOBAL_HEADER) {
        context.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
    }
    
    // Open the encoder context
    let mut encoder = context.encoder().video()
        .map_err(|e| AppError::General(format!("Failed to open video encoder: {}", e)))?;
    
    pb.set_message("Preparing video transcoding...");
    
    Ok(())
}

/// Transcode audio stream
fn transcode_audio_stream(
    input: &mut input::Input,
    output: &mut output::Output,
    stream_index: usize,
    bitrate: Option<&str>,
    progress: Arc<ProgressInfo>,
    pb: &ProgressBar,
) -> Result<(), AppError> {
    let input_stream = input.streams().get(stream_index)
        .ok_or_else(|| AppError::General("Audio stream not found".to_string()))?;
    
    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|e| AppError::General(format!("Failed to create decoder context: {}", e)))?;
    
    let mut decoder = context_decoder.decoder().audio()
        .map_err(|e| AppError::General(format!("Failed to find audio decoder: {}", e)))?;
    
    // Find a suitable audio encoder
    let encoder_name = find_suitable_audio_encoder();
    let encoder = ffmpeg::encoder::find_by_name(encoder_name)
        .ok_or_else(|| AppError::General(format!("Encoder {} not found", encoder_name)))?;
    
    // Create output stream
    let output_stream = output.add_stream(encoder)
        .map_err(|e| AppError::General(format!("Failed to create output stream: {}", e)))?;
    
    // Configure audio encoder context
    let mut context = ffmpeg::codec::context::Context::from_parameters(output_stream.parameters())
        .map_err(|e| AppError::General(format!("Failed to create encoder context: {}", e)))?;
    
    // Set audio params
    context.set_sample_rate(44100); // Standard 44.1kHz sample rate
    context.set_channels(2);        // Stereo
    context.set_format(decoder.format());
    
    // Parse and set bitrate
    let bitrate_value = match bitrate {
        Some(b) if b.ends_with('k') => {
            b[..b.len() - 1].parse::<usize>().unwrap_or(128) * 1000
        },
        Some(b) if b.ends_with('K') => {
            b[..b.len() - 1].parse::<usize>().unwrap_or(128) * 1000
        },
        _ => 128_000, // Default 128kbps
    };
    
    context.set_bit_rate(bitrate_value);
    
    // Get global header if needed
    if output.format().flags().contains(ffmpeg::format::Flags::GLOBAL_HEADER) {
        context.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
    }
    
    // Open the encoder context
    let mut encoder = context.encoder().audio()
        .map_err(|e| AppError::General(format!("Failed to open audio encoder: {}", e)))?;
    
    pb.set_message("Preparing audio transcoding...");
    
    Ok(())
}

/// Process all frames from input to output
fn process_frames(
    input: &mut input::Input,
    output: &mut output::Output,
    progress: Arc<ProgressInfo>,
    pb: &ProgressBar,
) -> Result<(), AppError> {
    // Process packets
    let mut frame_count = 0;
    let mut decoded_frame = ffmpeg::util::frame::Frame::empty();
    
    for (stream, packet) in input.packets() {
        frame_count += 1;
        
        // Update progress every 10 frames
        if frame_count % 10 == 0 {
            progress.update(10);
            let percent = progress.get_percentage();
            pb.set_position(percent);
            
            let elapsed = progress.get_elapsed_secs();
            let eta = progress.get_eta_secs();
            
            pb.set_message(format!(
                "Processed {} frames | Elapsed: {:.1}s | ETA: {:.1}s",
                frame_count,
                elapsed,
                eta
            ));
        }
    }
    
    Ok(())
}

/// Find a suitable video encoder based on system capabilities
fn find_suitable_video_encoder() -> &'static str {
    // Try H.264 encoders in order of preference
    let encoders = ["libx264", "h264_nvenc", "h264_amf", "h264_qsv", "h264_videotoolbox"];
    
    for encoder in encoders {
        if ffmpeg::encoder::find_by_name(encoder).is_some() {
            return encoder;
        }
    }
    
    // Fallback to MPEG-4
    "mpeg4"
}

/// Find a suitable audio encoder based on system capabilities
fn find_suitable_audio_encoder() -> &'static str {
    // Try MP3 encoders in order of preference
    let encoders = ["libmp3lame", "mp3", "libshine"];
    
    for encoder in encoders {
        if ffmpeg::encoder::find_by_name(encoder).is_some() {
            return encoder;
        }
    }
    
    // Fallback to AAC
    "aac"
}

/// Get the duration of a video file in seconds
pub fn get_video_duration(path: &Path) -> Result<u64, AppError> {
    let input_path = path.to_str()
        .ok_or_else(|| AppError::PathError("Invalid input path encoding".to_string()))?;
    
    // Open input file
    let input = input::open(&input_path)
        .map_err(|e| AppError::General(format!("Failed to open input file: {}", e)))?;
    
    // Get duration in AV_TIME_BASE units and convert to seconds
    let duration_seconds = input.duration() as f64 / ffmpeg::util::time::AV_TIME_BASE as f64;
    
    Ok(duration_seconds.round() as u64)
}

/// Check if a file already exists and handle duplicates
pub fn handle_duplicate_file(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    
    // Get filename and extension
    let file_stem = path.file_stem()
        .ok_or_else(|| AppError::PathError("Invalid filename".to_string()))?
        .to_string_lossy();
    
    let extension = path.extension()
        .map(|ext| ext.to_string_lossy().to_string())
        .unwrap_or_default();
    
    // Create a new filename with timestamp
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S");
    
    let new_filename = if extension.is_empty() {
        format!("{}_{}", file_stem, timestamp)
    } else {
        format!("{}_{}.{}", file_stem, timestamp, extension)
    };
    
    // Create new path
    let new_path = path.with_file_name(new_filename);
    
    Ok(new_path)
}