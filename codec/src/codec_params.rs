use ac_ffmpeg::codec::audio::{AudioFrame, ChannelLayout};
use ac_ffmpeg::codec::AudioCodecParameters;
use derive_builder::Builder;

use crate::SampleFormat;

#[derive(Debug, Copy, Clone, Builder)]
pub struct CodecParams {
    pub(crate) sample_rate: u32,
    pub(crate) sample_format: SampleFormat,
    #[builder(default = "1")]
    pub(crate) channels: u32,
    #[builder(default)]
    pub(crate) bit_rate: u64,
    #[builder(default)]
    pub(crate) samples_per_frame: Option<usize>,
}

impl CodecParams {
    pub const fn new(sample_rate: u32, sample_format: SampleFormat, channels: u32) -> Self {
        Self {
            sample_rate,
            sample_format,
            channels,
            bit_rate: 0,
            samples_per_frame: None,
        }
    }

    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub const fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn channel_layout(&self) -> ChannelLayout {
        ChannelLayout::from_channels(self.channels).expect("Valid channel layout")
    }

    pub const fn bit_rate(&self) -> u64 {
        self.bit_rate
    }

    pub const fn samples_per_frame(&self) -> Option<usize> {
        self.samples_per_frame
    }
}

impl From<&AudioCodecParameters> for CodecParams {
    fn from(params: &AudioCodecParameters) -> Self {
        Self {
            sample_rate: params.sample_rate(),
            sample_format: params.sample_format().into(),
            channels: params.channel_layout().channels(),
            bit_rate: params.bit_rate(),
            samples_per_frame: None,
        }
    }
}

impl From<&AudioFrame> for CodecParams {
    fn from(frame: &AudioFrame) -> Self {
        Self {
            sample_rate: frame.sample_rate(),
            sample_format: frame.sample_format().into(),
            channels: frame.channel_layout().channels(),
            bit_rate: 0u64,
            samples_per_frame: None,
        }
    }
}
