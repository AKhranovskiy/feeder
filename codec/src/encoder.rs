use std::io::Write;

use ac_ffmpeg::codec::audio::AudioEncoder;
use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::codec::audio::ChannelLayout;
use ac_ffmpeg::codec::Encoder as AsEncoder;
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::format::muxer::Muxer;
use ac_ffmpeg::format::muxer::OutputFormat;
use anyhow::anyhow;

use crate::CodecParams;
use crate::SampleFormat;

static OPUS_SAMPLE_RATE: u32 = 48_000;
static OPUS_SAMPLE_FORMAT: SampleFormat = SampleFormat::Flt;

#[non_exhaustive]
pub struct Encoder<T> {
    encoder: AudioEncoder,
    muxer: Muxer<T>,
}

#[inline(always)]
fn to_channel_layout(channels: u32) -> anyhow::Result<ChannelLayout> {
    ChannelLayout::from_channels(channels)
        .ok_or_else(|| anyhow!("Invalid channels number {channels}"))
}

impl<W: Write> Encoder<W> {
    pub fn opus(bit_rate: u64, channels: u32, output: W) -> anyhow::Result<Self> {
        let encoder = AudioEncoder::builder("libopus")?
            .sample_rate(OPUS_SAMPLE_RATE)
            .sample_format(OPUS_SAMPLE_FORMAT.into())
            .bit_rate(bit_rate)
            .channel_layout(to_channel_layout(channels)?)
            .build()?;

        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&encoder.codec_parameters().into())?;

        let muxer = muxer_builder.build(
            IO::from_write_stream(output),
            OutputFormat::find_by_name("ogg").expect("output format"),
        )?;

        Ok(Self { encoder, muxer })
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<&mut Self> {
        self.encoder.try_push(frame)?;
        while let Some(frame) = self.encoder.take()? {
            self.muxer.push(frame)?;
        }

        Ok(self)
    }

    pub fn flush(&mut self) -> anyhow::Result<&mut Self> {
        self.encoder.try_flush()?;

        while let Some(frame) = self.encoder.take()? {
            self.muxer.push(frame)?;
        }

        self.muxer.flush()?;

        Ok(self)
    }

    pub fn codec_params(&self) -> CodecParams {
        let params = self.encoder.codec_parameters();
        CodecParams {
            sample_rate: params.sample_rate(),
            sample_format: params.sample_format().into(),
            channels: params.channel_layout().channels(),
            bit_rate: params.bit_rate(),
            samples_per_frame: self.encoder.samples_per_frame(),
        }
    }
}
