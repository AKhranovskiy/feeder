use std::io::Write;

use codec::{CodecParams, SampleFormat};

fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Expects audio file");

    let io = std::io::BufReader::new(std::fs::File::open(input)?);

    let resampled = codec::resample(io, CodecParams::new(22050, SampleFormat::S16, 1))?;

    std::io::stdout().write_all(&resampled)?;
    std::io::stdout().flush()?;

    Ok(())
}
