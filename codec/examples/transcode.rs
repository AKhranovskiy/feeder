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

const OPUS_SAMPLE_RATE: u32 = 48_000;

/**
 * Takes stdin, decode, analyze, put encoded/muxed stream to stdin and analysis result to stderr.
 */
fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Expects audio file");

    let io = std::io::BufReader::new(std::fs::File::open(input)?);

    let decoder = Decoder::try_from(io)?;

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
        .source_channel_layout(params.channel_layout())
        .target_frame_samples(encoder.samples_per_frame())
        .target_sample_rate(OPUS_SAMPLE_RATE)
        .target_sample_format(SampleFormat::Flt.into())
        .target_channel_layout(params.channel_layout())
        .build()?;

    let mut muxer = {
        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&encoder.codec_parameters().into())?;
        muxer_builder.build(
            IO::from_write_stream(std::io::stdout()),
            OutputFormat::find_by_name("ogg").expect("output format"),
        )?
    };

    for frame in decoder {
        resampler.push(frame?)?;

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
