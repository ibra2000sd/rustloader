mod cli;
mod downloader;
mod error;
mod utils;

use cli::build_cli;
use colored::*;
use downloader::download_video;
use error::AppError;
use utils::{check_dependencies, install_ffmpeg};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    println!("{}", "Rustloader - Advanced Video Downloader".bright_cyan().bold());
    
    // Check for required dependencies
    match check_dependencies() {
        Ok(_) => (),
        Err(AppError::MissingDependency(dep)) if dep == "ffmpeg" => {
            println!("{}", "ffmpeg is not installed. Would you like to install it now? (y/n):".yellow());
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                install_ffmpeg()?;
            } else {
                println!("{}", "Please install ffmpeg manually and try again.".red());
                return Err(AppError::MissingDependency("ffmpeg installation declined".to_string()));
            }
        },
        Err(e) => return Err(e),
    }

    // Parse command-line arguments
    let matches = build_cli().get_matches();
    
    // Extract URL and options
    let url = matches.get_one::<String>("url").unwrap();
    let quality = matches.get_one::<String>("quality").map(|q| q.as_str());
    let format = matches.get_one::<String>("format").map(|f| f.as_str()).unwrap_or("mp4");
    let start_time = matches.get_one::<String>("start-time");
    let end_time = matches.get_one::<String>("end-time");
    let use_playlist = matches.get_flag("playlist");
    let download_subtitles = matches.get_flag("subtitles");
    let output_dir = matches.get_one::<String>("output-dir");
    
    // Perform video download
    match download_video(
        url, 
        quality, 
        format, 
        start_time, 
        end_time, 
        use_playlist,
        download_subtitles,
        output_dir,
    ).await {
        Ok(_) => println!("{}", "Process completed successfully.".green()),
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}