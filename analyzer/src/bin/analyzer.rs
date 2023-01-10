use std::io::{BufWriter, Write};

use ac_ffmpeg::codec::audio::AudioFrameMut;
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

        let class = analyzer.push(frame.clone())?;
        if class == Some("A") {
            let silence = AudioFrameMut::silence(
                frame.channel_layout(),
                frame.sample_format(),
                frame.sample_rate(),
                frame.samples(),
            )
            .freeze();
            encoder.push(silence)?;
        } else {
            encoder.push(frame)?;
        }

        std::io::stderr().write_all(class.unwrap_or("").as_bytes())?;
    }

    encoder.flush()?;

    Ok(())
}
