use std::io::{BufWriter, Write};
use std::str::FromStr;

use ac_ffmpeg::codec::audio::AudioFrameMut;
use codec::{Decoder, Encoder};

use analyzer::LabelSmoother;
use analyzer::{BufferedAnalyzer, ContentKind};

fn main() -> anyhow::Result<()> {
    let url = url::Url::from_str(&std::env::args().nth(1).expect("Expects URL"))?;

    let input = unstreamer::Unstreamer::open(url)?;

    let decoder = Decoder::try_from(input)?;

    let mut encoder = Encoder::opus(decoder.codec_params(), BufWriter::new(std::io::stdout()))?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(5));

    for frame in decoder {
        let frame = frame?;

        let kind = analyzer.push(frame.clone())?;
        if kind == ContentKind::Advertisement {
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

        std::io::stderr().write_all(&kind.name().as_bytes()[..1])?;
    }

    encoder.flush()?;

    Ok(())
}
