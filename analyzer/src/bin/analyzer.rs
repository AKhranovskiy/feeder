use std::io::Write;

use ac_ffmpeg::{
    codec::{
        audio::{AudioEncoder, AudioResampler},
        Encoder,
    },
    format::{
        io::IO,
        muxer::{Muxer, OutputFormat},
    },
};

use codec::{Decoder, SampleFormat};

use analyzer::BufferedAnalyzer;
use analyzer::LabelSmoother;

const OPUS_SAMPLE_RATE: u32 = 48_000;

/**
 * Takes stdin, decode, analyze, put encoded/muxed stream to stdin and analysis result to stderr.
 */
fn main() -> anyhow::Result<()> {
    let decoder = Decoder::try_from(std::io::stdin())?;
    let params = decoder.codec_parameters();

    let mut encoder = AudioEncoder::builder("libopus")?
        .sample_rate(OPUS_SAMPLE_RATE)
        .bit_rate(params.bit_rate())
        .sample_format(SampleFormat::Flt.into())
        .channel_layout(params.channel_layout().to_owned())
        .build()?;

    let mut resampler = AudioResampler::builder()
        .source_sample_rate(params.sample_rate())
        .source_sample_format(params.sample_format())
        .source_channel_layout(params.channel_layout().to_owned())
        .target_frame_samples(encoder.samples_per_frame())
        .target_sample_rate(OPUS_SAMPLE_RATE)
        .target_sample_format(SampleFormat::Flt.into())
        .target_channel_layout(params.channel_layout().to_owned())
        .build()?;

    let mut muxer = {
        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&encoder.codec_parameters().into())?;
        muxer_builder.build(
            IO::from_write_stream(std::io::stdout()),
            OutputFormat::find_by_name("ogg").expect("output format"),
        )?
    };

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(5));

    for frame in decoder {
        let frame = frame?;

        if let Some(class) = analyzer.push(frame.clone())? {
            std::io::stderr().write_all(class.as_bytes())?;
        }

        resampler.push(frame)?;

        while let Some(frame) = resampler.take()? {
            encoder.push(frame)?;

            while let Some(encoded_frame) = encoder.take()? {
                muxer.push(encoded_frame)?;
            }
        }
    }

    encoder.flush()?;
    while let Some(encoded_frame) = encoder.take()? {
        muxer.push(encoded_frame)?;
    }

    muxer.flush()?;

    Ok(())
}
