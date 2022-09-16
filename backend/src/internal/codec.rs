use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::anyhow;

use rocket::http::ContentType;

pub fn prepare_for_browser(
    content_type: &ContentType,
    content: &[u8],
) -> anyhow::Result<(ContentType, Vec<u8>)> {
    if content_type.is_aac() {
        remux_aac(content)
    } else {
        Ok((content_type.clone(), content.to_vec()))
    }
}

fn remux_aac(bytes: &[u8]) -> anyhow::Result<(ContentType, Vec<u8>)> {
    let ffmpeg_path = std::env::var("FFMPEG_PATH")?;
    log::info!("FFMPEG_PATH: {}", ffmpeg_path);

    let args = "-i pipe: -vn -c copy -map_metadata 0 -movflags +faststart -f adts pipe:";
    log::debug!("FFMPEG ARGS: {args}");

    let proc = Command::new("ffmpeg")
        .env("PATH", ffmpeg_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args(args.split_ascii_whitespace())
        .spawn()?;

    proc.stdin
        .as_ref()
        .ok_or_else(|| anyhow!("Failed to acquire stdin"))
        .and_then(|mut stdin| stdin.write_all(bytes).map_err(|e| e.into()))?;

    Ok((ContentType::AAC, proc.wait_with_output()?.stdout))
}
