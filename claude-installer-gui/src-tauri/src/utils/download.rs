use futures_util::StreamExt;
use reqwest::Client;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Progress callback data sent during download
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    /// Bytes per second
    pub speed_bps: u64,
    /// Estimated time remaining in seconds
    pub eta_secs: u64,
}

/// Download a file from a URL to a destination path, reporting progress via callback.
/// The callback is called approximately every 100ms to avoid flooding the frontend.
pub async fn download_file<F>(
    url: &str,
    dest: &Path,
    on_progress: F,
) -> Result<(), String>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    let client = Client::builder()
        .timeout(Duration::from_secs(600)) // 10 minute total timeout
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();

    let mut file = File::create(dest)
        .await
        .map_err(|e| format!("Failed to create file '{}': {}", dest.display(), e))?;

    let mut downloaded: u64 = 0;
    let start = Instant::now();
    let mut last_progress = Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        downloaded += chunk.len() as u64;

        // Throttle progress updates to ~10 per second
        if last_progress.elapsed() >= Duration::from_millis(100) || downloaded >= total {
            let elapsed = start.elapsed().as_secs_f64();
            let speed_bps = if elapsed > 0.0 {
                (downloaded as f64 / elapsed) as u64
            } else {
                0
            };
            let eta_secs = if speed_bps > 0 && total > 0 {
                ((total - downloaded) as f64 / speed_bps as f64) as u64
            } else {
                0
            };

            on_progress(DownloadProgress {
                downloaded,
                total,
                speed_bps,
                eta_secs,
            });

            last_progress = Instant::now();
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;

    Ok(())
}

/// Download a file with retry logic. Retries up to `max_retries` times with exponential backoff.
pub async fn download_with_retry<F>(
    url: &str,
    dest: &Path,
    max_retries: u32,
    on_progress: F,
    on_retry: impl Fn(u32, u32, &str),
) -> Result<(), String>
where
    F: Fn(DownloadProgress) + Send + Clone + 'static,
{
    let backoff_ms = [1000, 3000, 9000];

    for attempt in 0..=max_retries {
        match download_file(url, dest, on_progress.clone()).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt < max_retries {
                    let wait = backoff_ms.get(attempt as usize).copied().unwrap_or(9000);
                    on_retry(attempt + 1, max_retries, &e);
                    tokio::time::sleep(Duration::from_millis(wait as u64)).await;
                    // Clean up partial download
                    let _ = tokio::fs::remove_file(dest).await;
                } else {
                    return Err(format!(
                        "Download failed after {} attempts. Last error: {}",
                        max_retries + 1,
                        e
                    ));
                }
            }
        }
    }

    unreachable!()
}

/// Format bytes into human-readable string (e.g., "45.2 MB")
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format speed in bytes/sec to human-readable (e.g., "12.5 MB/s")
pub fn format_speed(bps: u64) -> String {
    format!("{}/s", format_bytes(bps))
}
