use std::io::{Read, Seek};
use std::str::FromStr;

use ac_ffmpeg::codec::audio::{AudioDecoder, AudioResampler, ChannelLayout, SampleFormat};
use ac_ffmpeg::codec::Decoder;
use ac_ffmpeg::format::demuxer::Demuxer;
use ac_ffmpeg::format::io::IO;
use bytemuck::cast_slice;

pub fn decode<RS>(input: RS) -> anyhow::Result<Vec<i16>>
where
    RS: Read + Seek,
{
    let io = IO::from_seekable_read_stream(input);

    let mut demuxer = Demuxer::builder()
        .build(io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)?;

    let mut decoder = AudioDecoder::from_stream(&demuxer.streams()[0])?.build()?;

    let codec = demuxer.streams()[0].codec_parameters();
    let params = codec.as_audio_codec_parameters().unwrap();

    let mut resampler = AudioResampler::builder()
        .source_sample_rate(params.sample_rate())
        .source_sample_format(params.sample_format())
        .source_channel_layout(params.channel_layout())
        .target_sample_rate(22050)
        .target_sample_format(SampleFormat::from_str("s16").expect("Sample format"))
        .target_channel_layout(ChannelLayout::from_channels(1).expect("Mono channel layout"))
        .build()?;

    let mut output = vec![];

    while let Some(packet) = demuxer.take()? {
        decoder.push(packet)?;
        while let Some(frame) = decoder.take()? {
            resampler.push(frame)?;
            while let Some(frame) = resampler.take()? {
                output.extend_from_slice(frame.planes()[0].data());
            }
        }
    }

    decoder.flush()?;

    while let Some(frame) = decoder.take()? {
        resampler.push(frame)?;
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
