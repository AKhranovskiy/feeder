use std::{
    env::args,
    fs::File,
    io::{BufReader, BufWriter},
};

use codec::{Decoder, Encoder};

fn main() -> anyhow::Result<()> {
    let input = args().nth(1).expect("Expects input file");

    let decoder = Decoder::try_from(BufReader::new(File::open(input)?))?;
    let mut encoder = Encoder::aac(decoder.codec_params(), BufWriter::new(std::io::stdout()))?;

    for frame in decoder {
        encoder.push(frame?)?;
    }

    encoder.flush()?;

    Ok(())
}
