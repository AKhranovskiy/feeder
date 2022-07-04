use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::anyhow;

use bytes::Bytes;
use rocket::http::ContentType;

pub fn prepare_for_browser(content_type: &ContentType, content: &Bytes) -> anyhow::Result<Vec<u8>> {
    if content_type.is_aac() {
        remux_aac(content).map(|bytes| bytes.to_vec())
    } else {
        Ok(content.to_vec())
    }
}

fn remux_aac(bytes: &Bytes) -> anyhow::Result<Bytes> {
    let ffmpeg_path = std::env::var("FFMPEG_PATH")?;
    log::info!("FFMPEG_PATH: {}", ffmpeg_path);

    let proc = Command::new("ffmpeg")
        .env("PATH", ffmpeg_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args([
            "-i", "pipe:", "-vn", "-acodec", "copy", "-f", "adts", "pipe:",
        ])
        .spawn()?;

    proc.stdin
        .as_ref()
        .ok_or_else(|| anyhow!("Failed to acquire stdin"))
        .and_then(|mut stdin| stdin.write_all(bytes).map_err(|e| e.into()))?;

    let remuxed: Bytes = proc.wait_with_output()?.stdout.into();
    Ok(remuxed)
}
