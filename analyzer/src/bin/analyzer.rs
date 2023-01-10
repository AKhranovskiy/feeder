use std::io::{BufWriter, Write};

use codec::{Decoder, Encoder};

use analyzer::BufferedAnalyzer;
use analyzer::LabelSmoother;

/**
 * Takes stdin, decode, analyze, put encoded/muxed stream to stdin and analysis result to stderr.
 */
fn main() -> anyhow::Result<()> {
    let decoder = Decoder::try_from(std::io::stdin())?;

    let mut encoder = Encoder::opus(decoder.codec_params(), BufWriter::new(std::io::stdout()))?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(5));

    for frame in decoder {
        let frame = frame?;

        if let Some(class) = analyzer.push(frame.clone())? {
            std::io::stderr().write_all(class.as_bytes())?;
        }

        encoder.push(frame)?;
    }

    encoder.flush()?;

    Ok(())
}
