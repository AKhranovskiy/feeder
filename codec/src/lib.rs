use std::io::Read;

pub use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::codec::audio::AudioFrameMut;
pub use ac_ffmpeg::packet::Packet;
use bytemuck::cast_slice;

mod frame_ext;
pub use frame_ext::FrameDuration;

mod decoder;
pub use decoder::Decoder;

mod encoder;
pub use encoder::Encoder;

mod resampler;
pub use resampler::{Resampler, ResamplingDecoder};

mod sample_format;
pub use sample_format::SampleFormat;

mod codec_params;
pub use codec_params::{CodecParams, CodecParamsBuilder};

pub mod dsp;

// TODO Sample should be bound to SampleFormat.
pub fn resample<R, Sample>(input: R, target: CodecParams) -> anyhow::Result<Vec<Sample>>
where
    R: Read,
    Sample: Clone + bytemuck::Pod,
{
    let decoder = Decoder::try_from(input)?.resample(target);

    let mut output: Vec<Sample> = vec![];

    for frame in decoder {
        for plane in frame?.planes().iter() {
            output.extend_from_slice(cast_slice::<u8, Sample>(plane.data()));
        }
    }

    Ok(output)
}

pub fn silence_frame(frame: &AudioFrame) -> AudioFrame {
    AudioFrameMut::silence(
        frame.channel_layout(),
        frame.sample_format(),
        frame.sample_rate(),
        frame.samples(),
    )
    .freeze()
}
