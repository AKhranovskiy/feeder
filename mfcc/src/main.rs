use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{anyhow, Result};
use mfcc::{calculate_mel_coefficients_with_deltas, ffmpeg_decode, plot, SAMPLE_RATE};

fn main() -> Result<()> {
    let file = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("File name is required"))?;

    let content = load_file_content(file)?;
    let samples = ffmpeg_decode(&content)?;

    println!(
        "Sample rate: {SAMPLE_RATE}Hz, # samples: {}, length: {:0.2}",
        samples.len(),
        samples.len() as f64 / f64::from(SAMPLE_RATE)
    );

    let mfccs = calculate_mel_coefficients_with_deltas(&samples)?;

    println!("MFCC Shape: {:?}", mfccs.shape());
    plot(&mfccs, "mfccs");

    Ok(())
}

fn load_file_content<P>(filename: P) -> Result<Vec<u8>>
where
    P: AsRef<Path>,
{
    let mut buffer = Vec::new();
    File::open(filename)?.read_to_end(&mut buffer)?;
    Ok(buffer)
}
