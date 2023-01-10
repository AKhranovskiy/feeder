use std::str::FromStr;

pub use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::codec::audio::{AudioResampler, ChannelLayout, SampleFormat as AcSampleFormat};
use ac_ffmpeg::codec::AudioCodecParameters;
use derive_builder::Builder;

use crate::Decoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    S16,
    Flt,
    FltPlanar,
}

impl From<SampleFormat> for AcSampleFormat {
    fn from(format: SampleFormat) -> Self {
        match format {
            SampleFormat::S16 => AcSampleFormat::from_str("s16").expect("s16"),
            SampleFormat::Flt => AcSampleFormat::from_str("flt").expect("flt"),
            SampleFormat::FltPlanar => AcSampleFormat::from_str("fltp").expect("flt"),
        }
    }
}

impl From<AcSampleFormat> for SampleFormat {
    fn from(format: AcSampleFormat) -> Self {
        match format.name() {
            "s16" => SampleFormat::S16,
            "flt" => SampleFormat::Flt,
            "fltp" => SampleFormat::FltPlanar,
            x => unreachable!("Unknown format {}", x),
        }
    }
}

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

#[non_exhaustive]
pub struct Resampler(AudioResampler);

impl Resampler {
    pub fn new(source: CodecParams, target: CodecParams) -> Self {
        Self(
            AudioResampler::builder()
                .source_sample_rate(source.sample_rate())
                .source_sample_format(source.sample_format().into())
                .source_channel_layout(source.channel_layout())
                .target_frame_samples(target.samples_per_frame())
                .target_sample_rate(target.sample_rate())
                .target_sample_format(target.sample_format().into())
                .target_channel_layout(target.channel_layout())
                .build()
                .expect("Resample setup is complete"),
        )
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<&mut Self> {
        self.0.try_push(frame).map(|_| self).map_err(Into::into)
    }
}

impl Iterator for Resampler {
    type Item = anyhow::Result<AudioFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map_err(Into::into).transpose()
        // .or_else(|| {
        //     self.0
        //         .flush()
        //         .map_err(Into::into)
        //         .and_then(|_| self.0.take().map_err(Into::into))
        //         .transpose()
        // })
    }
}

pub struct ResamplingDecoder<T> {
    decoder: Decoder<T>,
    resampler: Resampler,
}

impl<T> ResamplingDecoder<T> {
    pub(crate) fn new(decoder: Decoder<T>, target: CodecParams) -> Self {
        let source = CodecParams::from(&decoder.codec_parameters());
        let resampler = Resampler::new(source, target);
        Self { decoder, resampler }
    }
}

impl<T> Iterator for ResamplingDecoder<T> {
    type Item = anyhow::Result<AudioFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        // Is there anything in resampler?
        match self.resampler.next() {
            Some(Ok(frame)) => return Some(Ok(frame)),
            Some(Err(error)) => return Some(Err(error)),
            None => {}
        };

        match self.decoder.next() {
            Some(Ok(frame)) => match self.resampler.push(frame) {
                Ok(_) => return self.resampler.next(),
                Err(error) => return Some(Err(error)),
            },
            Some(Err(error)) => return Some(Err(error)),
            None => {}
        }

        None
    }
}
