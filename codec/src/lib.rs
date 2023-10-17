use std::io::{Cursor, Read};
use std::str::FromStr;
use std::time::Duration;

pub use ac_ffmpeg::codec::audio::AudioFrame;
pub use ac_ffmpeg::packet::Packet;
pub use ac_ffmpeg::time::{TimeBase, Timestamp};

use ac_ffmpeg::{
    codec::audio::AudioFrameMut,
    codec::audio::ChannelLayout as AcChannelLayout,
    codec::audio::SampleFormat as AcSampleFormat,
    codec::audio::{AudioDecoder, AudioResampler},
    codec::Decoder as AcDecoder,
    format::demuxer::Demuxer,
    format::io::IO,
    set_log_callback,
};

use bytemuck::cast_slice;

mod frame_ext;
pub use frame_ext::FrameDuration;

mod decoder;
pub use decoder::Decoder;

mod encoder;
pub use encoder::Encoder;

mod resampler;
use log::Level;
pub use resampler::{Resampler, ResamplingDecoder};

mod sample_format;
pub use sample_format::SampleFormat;

mod codec_params;
pub use codec_params::{CodecParams, CodecParamsBuilder};

pub mod dsp;

mod pts;
pub use pts::Pts;

pub fn suppress_ffmpeg_log() {
    set_log_callback(|_, _| {});
}

pub fn configure_ffmpeg_log() {
    set_log_callback(|level, message| {
        let level = match level {
            // Quiet
            -8 => return,
            // FFMpeg is going to crash.
            0 => panic!("FFMPEG panics: {message}"),
            // PANIC and ERROR
            8 | 16 => Level::Error,
            24 => Level::Warn,
            32 => Level::Info,
            // VERBOSE
            40 => Level::Debug,
            // DEBUG
            48 => Level::Trace,
            _ => unreachable!("Unknown log level {} {}", level, message),
        };
        log::log!(level, "FFMPEG: {}", message);
    });
}

pub fn resample_16k_mono_s16_stream<R: Read>(input: R) -> anyhow::Result<Vec<i16>> {
    let io = IO::from_read_stream(input);

    let mut demuxer = Demuxer::builder()
        .build(io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)?;

    let params = demuxer.streams()[0].codec_parameters();
    let source = params.as_audio_codec_parameters().unwrap();

    let mut decoder = AudioDecoder::from_stream(&demuxer.streams()[0])?.build()?;

    let mut resampler = AudioResampler::builder()
        .source_sample_rate(source.sample_rate())
        .source_channel_layout(source.channel_layout().to_owned())
        .source_sample_format(source.sample_format())
        .source_sample_rate(source.sample_rate())
        .target_channel_layout(AcChannelLayout::from_channels(1).unwrap())
        .target_sample_format(AcSampleFormat::from_str("s16").unwrap())
        .target_sample_rate(16_000)
        .build()?;

    let mut output: Vec<i16> = vec![];

    while let Some(packet) = demuxer.take()? {
        decoder.push(packet)?;
        while let Some(frame) = decoder.take()? {
            resampler.push(frame)?;
            while let Some(frame) = resampler.take()? {
                output.extend_from_slice(cast_slice(frame.planes()[0].data()));
            }
        }
    }

    decoder.flush()?;
    while let Some(frame) = decoder.take()? {
        resampler.push(frame)?;
        while let Some(frame) = resampler.take()? {
            output.extend_from_slice(cast_slice(frame.planes()[0].data()));
        }
    }

    resampler.flush()?;
    while let Some(frame) = resampler.take()? {
        output.extend_from_slice(cast_slice(frame.planes()[0].data()));
    }

    Ok(output)
}

pub fn resample_16k_mono_s16_frames(frames: Vec<AudioFrame>) -> anyhow::Result<Vec<i16>> {
    if frames.is_empty() {
        return Ok(vec![]);
    }

    let mut output: Vec<i16> = vec![];

    let mut resampler = match AudioResampler::builder()
        .source_sample_rate(frames[0].sample_rate())
        .source_channel_layout(frames[0].channel_layout().to_owned())
        .source_sample_format(frames[0].sample_format())
        .source_sample_rate(frames[0].sample_rate())
        .target_channel_layout(AcChannelLayout::from_channels(1).unwrap())
        .target_sample_format(AcSampleFormat::from_str("s16").unwrap())
        .target_sample_rate(16_000)
        .build()
    {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Failed to create resampler: {err}");
            anyhow::bail!(err);
        }
    };

    for frame in frames {
        resampler.push(frame)?;

        // Plane data buffer is bigger than required amount,
        // so take only required amount.
        while let Some(frame) = resampler.take()? {
            // data() returns &[u8], and contains i16, so multiply number of samples by 2.
            output.extend_from_slice(cast_slice(&frame.planes()[0].data()[..frame.samples() * 2]));
        }
    }

    resampler.flush()?;

    while let Some(frame) = resampler.take()? {
        // data() returns &[u8], and contains i16, so multiply number of samples by 2.
        output.extend_from_slice(cast_slice(&frame.planes()[0].data()[..frame.samples() * 2]));
    }

    Ok(output)
}

#[must_use]
pub fn silence_frame(frame: &AudioFrame) -> AudioFrame {
    AudioFrameMut::silence(
        frame.channel_layout(),
        frame.sample_format(),
        frame.sample_rate(),
        frame.samples(),
    )
    .freeze()
}

pub fn track_duration(input: &[u8]) -> anyhow::Result<Duration> {
    Ok(Decoder::try_from(Cursor::new(input))?
        .take_while(Result::is_ok)
        .map(Result::unwrap)
        .map(|frame| frame.duration())
        .sum())
}

pub fn track_codec_params(input: &[u8]) -> anyhow::Result<CodecParams> {
    Ok(Decoder::try_from(Cursor::new(input))?.codec_params())
}
