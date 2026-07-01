//! Cross-session resume identity guard (F-DL-003, Shape 2).
//!
//! #28/#29 made a segment's resume-from-written-bytes safe against a range
//! that's silently ignored by the server, but said nothing about whether the
//! `.partN` files on disk actually belong to *this* download's plan. A
//! segment-count change between sessions, or a different download reusing
//! the same `output_path`, would otherwise get silently appended into. This
//! module records the identity a set of parts was written for in a small
//! sidecar file next to them, so the engine can require a match before
//! trusting any existing part.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Bumped whenever the sidecar format changes; a version mismatch compares
/// unequal like any other field mismatch, so it's handled by the same
/// safe-restart path as every other kind of mismatch, never as a parse error.
const SCHEMA_VERSION: u32 = 1;

/// Identity of the download a set of `.partN` files were written for.
/// Compared against the sidecar on disk before any cross-session resume is
/// trusted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeIdentity {
    schema_version: u32,
    url_hash: u64,
    file_size: u64,
    segment_count: usize,
}

impl ResumeIdentity {
    pub fn new(url: &str, file_size: u64, segment_count: usize) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            url_hash: hash_url(url),
            file_size,
            segment_count,
        }
    }
}

fn hash_url(url: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    hasher.finish()
}

/// Path of the sidecar identity file for a given output path, placed next to
/// the output (and the `.partN` files) the same way `calculate_segments`
/// derives part paths.
pub fn sidecar_path(output_path: &Path) -> PathBuf {
    let mut name = output_path.file_name().unwrap_or_default().to_os_string();
    name.push(".rustloader-resume");
    match output_path.parent() {
        Some(parent) => parent.join(&name),
        None => PathBuf::from(&name),
    }
}

/// Best-effort read: a missing file, an I/O error, or corrupt/incompatible
/// JSON are all treated as "no identity on record" so the caller falls back
/// to the safe clean-restart path instead of erroring the download.
pub async fn read_sidecar(path: &Path) -> Option<ResumeIdentity> {
    let bytes = tokio::fs::read(path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Best-effort write; failures are returned for the caller to log, not to
/// abort the download over (mirrors `EventLog`'s failure-tolerant writes).
pub async fn write_sidecar(path: &Path, identity: &ResumeIdentity) -> Result<()> {
    let json = serde_json::to_vec(identity)?;
    tokio::fs::write(path, json).await?;
    Ok(())
}

/// Best-effort delete; a missing sidecar is not an error.
pub async fn remove_sidecar(path: &Path) {
    if let Err(e) = tokio::fs::remove_file(path).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(
                "Failed to remove resume identity sidecar {}: {}",
                path.display(),
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_path_lives_next_to_output() {
        let out = Path::new("/tmp/dl/movie.mp4");
        let path = sidecar_path(out);
        assert_eq!(path.parent(), out.parent());
        assert_eq!(
            path.file_name().unwrap(),
            std::ffi::OsStr::new("movie.mp4.rustloader-resume")
        );
    }

    #[test]
    fn identity_equality_is_field_wise() {
        let a = ResumeIdentity::new("https://example.com/a.mp4", 1000, 4);
        let b = ResumeIdentity::new("https://example.com/a.mp4", 1000, 4);
        let different_url = ResumeIdentity::new("https://example.com/b.mp4", 1000, 4);
        let different_size = ResumeIdentity::new("https://example.com/a.mp4", 2000, 4);
        let different_segments = ResumeIdentity::new("https://example.com/a.mp4", 1000, 8);

        assert_eq!(a, b);
        assert_ne!(a, different_url);
        assert_ne!(a, different_size);
        assert_ne!(a, different_segments);
    }

    #[tokio::test]
    async fn write_then_read_round_trips() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("out.mp4.rustloader-resume");
        let identity = ResumeIdentity::new("https://example.com/a.mp4", 12345, 8);

        write_sidecar(&path, &identity)
            .await
            .expect("write sidecar");
        assert_eq!(read_sidecar(&path).await, Some(identity));
    }

    #[tokio::test]
    async fn missing_sidecar_reads_as_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("does-not-exist.rustloader-resume");
        assert!(read_sidecar(&path).await.is_none());
    }

    #[tokio::test]
    async fn corrupt_sidecar_reads_as_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("out.mp4.rustloader-resume");
        tokio::fs::write(&path, b"not valid json")
            .await
            .expect("write garbage");
        assert!(read_sidecar(&path).await.is_none());
    }

    #[tokio::test]
    async fn remove_sidecar_is_a_noop_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("does-not-exist.rustloader-resume");
        remove_sidecar(&path).await; // must not panic or error
    }
}
