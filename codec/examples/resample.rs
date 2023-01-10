use std::io::Write;

use codec::{CodecParams, Decoder, SampleFormat};

fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Expects audio file");

    let io = std::io::BufReader::new(std::fs::File::open(input)?);

    let decoder = Decoder::try_from(io)?.resample(CodecParams::new(22050, SampleFormat::S16, 1));

    let mut resampled: Vec<u8> = vec![];

    for frame in decoder {
        for plane in frame?.planes().iter() {
            resampled.extend_from_slice(plane.data());
        }
    }

    std::io::stdout().write_all(&resampled)?;
    std::io::stdout().flush()?;

    Ok(())
}
