#!/usr/bin/env rust-script
//! Test Facebook URL extraction exactly as the app does
//! Run with: cargo run --release --bin test_facebook

use std::process::Command;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let fb_url = "https://www.facebook.com/reel/1187641342944587";
    
    // Find yt-dlp (same as app does)
    let ytdlp_path = which::which("yt-dlp")
        .expect("yt-dlp not found");
    
    println!("Testing Facebook URL extraction...");
    println!("URL: {}", fb_url);
    println!("yt-dlp: {:?}", ytdlp_path);
    println!();
    
    let output = tokio::process::Command::new(&ytdlp_path)
        .arg("--dump-json")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg(fb_url)
        .output()
        .await
        .expect("Failed to execute yt-dlp");
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("❌ EXTRACTION FAILED");
        eprintln!("Error: {}", error_msg);
        std::process::exit(1);
    }
    
    let json_str = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8 in output");
    
    // Try to parse as VideoInfo would
    println!("✅ Extraction succeeded");
    println!("JSON length: {} bytes", json_str.len());
    println!();
    
    // Check if it parses
    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(value) => {
            println!("✅ Valid JSON");
            println!("id: {}", value["id"]);
            println!("title: {}", value["title"]);
            println!("webpage_url: {}", value["webpage_url"]);
            println!();
            
            // Check required fields
            let has_id = value.get("id").is_some();
            let has_title = value.get("title").is_some();
            let has_url = value.get("webpage_url").is_some();
            
            if has_id && has_title && has_url {
                println!("✅ All required fields present");
            } else {
                println!("❌ Missing required fields:");
                if !has_id { println!("  - id"); }
                if !has_title { println!("  - title"); }
                if !has_url { println!("  - webpage_url"); }
            }
        }
        Err(e) => {
            eprintln!("❌ JSON parsing failed: {}", e);
            eprintln!("First 500 chars: {}", &json_str[..json_str.len().min(500)]);
        }
    }
}
