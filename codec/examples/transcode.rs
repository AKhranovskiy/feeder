use ac_ffmpeg::format::{
    io::IO,
    muxer::{Muxer, OutputFormat},
};

use codec::{CodecParamsBuilder, Decoder, Encoder, SampleFormat};

const OPUS_SAMPLE_RATE: u32 = 48_000;

/**
 * Takes stdin, decode, analyze, put encoded/muxed stream to stdin and analysis result to stderr.
 */
fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Expects audio file");

    let io = std::io::BufReader::new(std::fs::File::open(input)?);

    let decoder = Decoder::try_from(io)?;

    let params = decoder.codec_parameters();

    let mut encoder = Encoder::opus(
        CodecParamsBuilder::default()
            .sample_rate(OPUS_SAMPLE_RATE)
            .sample_format(SampleFormat::Flt)
            .channels(params.channel_layout().channels())
            .bit_rate(params.bit_rate())
            .build()?,
    )?;

    let decoder = decoder.resample(encoder.codec_params());

    let mut muxer = {
        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&encoder.codec_parameters())?;
        muxer_builder.build(
            IO::from_write_stream(std::io::stdout()),
            OutputFormat::find_by_name("ogg").expect("output format"),
        )?
    };

    for frame in decoder {
        for encoded_frame in encoder.push(frame?)? {
            muxer.push(encoded_frame?)?;
        }
    }
    for encoded_frame in encoder.flush()? {
        muxer.push(encoded_frame?)?;
    }

    muxer.flush()?;

    Ok(())
}
