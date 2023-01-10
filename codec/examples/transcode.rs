use std::{
    env::args,
    fs::File,
    io::{BufReader, BufWriter},
};

use codec::{Decoder, Encoder};

/**
 * Takes stdin, decode, analyze, put encoded/muxed stream to stdin and analysis result to stderr.
 */
fn main() -> anyhow::Result<()> {
    let input = args().nth(1).expect("Expects input file");

    let decoder = Decoder::try_from(BufReader::new(File::open(input)?))?;

    let params = decoder.codec_parameters();

    let mut encoder = Encoder::opus(
        params.bit_rate(),
        params.channel_layout().channels(),
        BufWriter::new(std::io::stdout()),
    )?;

    let decoder = decoder.resample(encoder.codec_params());

    for frame in decoder {
        encoder.push(frame?)?;
    }

    encoder.flush()?;

    Ok(())
}
