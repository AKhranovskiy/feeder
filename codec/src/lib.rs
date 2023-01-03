use std::io::{Read, Seek};
use std::str::FromStr;

use ac_ffmpeg::codec::audio::{AudioResampler, ChannelLayout, SampleFormat};
use bytemuck::cast_slice;

mod demuxer;
pub use demuxer::Demuxer;

mod decoder;
pub use decoder::Decoder;

pub fn decode<RS>(input: RS) -> anyhow::Result<Vec<i16>>
where
    RS: Read + Seek,
{
    let decoder = Decoder::try_from(input)?;
    let params = decoder.audio_codec_parameters().unwrap();

    let mut resampler = AudioResampler::builder()
        .source_sample_rate(params.sample_rate())
        .source_sample_format(params.sample_format())
        .source_channel_layout(params.channel_layout())
        .target_sample_rate(22050)
        .target_sample_format(SampleFormat::from_str("s16").expect("Sample format"))
        .target_channel_layout(ChannelLayout::from_channels(1).expect("Mono channel layout"))
        .build()?;

    let mut output = vec![];

    for frame in decoder {
        resampler.push(frame?)?;
        while let Some(frame) = resampler.take()? {
            output.extend_from_slice(frame.planes()[0].data());
        }
    }

    resampler.flush()?;

    while let Some(frame) = resampler.take()? {
        output.extend_from_slice(frame.planes()[0].data());
    }

    Ok(cast_slice::<u8, i16>(output.as_slice()).to_vec())
}
