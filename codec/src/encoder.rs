use ac_ffmpeg::codec::audio::AudioEncoder;
use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::codec::CodecParameters;
use ac_ffmpeg::codec::Encoder as AsEncoder;
use ac_ffmpeg::packet::Packet;

use crate::CodecParams;

#[non_exhaustive]
pub struct Encoder(AudioEncoder);

impl Encoder {
    pub fn opus(params: CodecParams) -> anyhow::Result<Self> {
        AudioEncoder::builder("libopus")?
            .sample_rate(params.sample_rate())
            .bit_rate(params.bit_rate())
            .sample_format(params.sample_format().into())
            .channel_layout(params.channel_layout())
            .build()
            .map(Self)
            .map_err(Into::into)
    }

    pub fn codec_parameters(&self) -> CodecParameters {
        self.0.codec_parameters().into()
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<&mut Self> {
        self.0.try_push(frame).map(|_| self).map_err(Into::into)
    }

    pub fn flush(&mut self) -> anyhow::Result<&mut Self> {
        self.0.try_flush().map(|_| self).map_err(Into::into)
    }

    pub fn codec_params(&self) -> CodecParams {
        let params = self.0.codec_parameters();
        CodecParams {
            sample_rate: params.sample_rate(),
            sample_format: params.sample_format().into(),
            channels: params.channel_layout().channels(),
            bit_rate: params.bit_rate(),
            samples_per_frame: self.0.samples_per_frame(),
        }
    }
}

impl Iterator for Encoder {
    type Item = anyhow::Result<Packet>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map_err(Into::into).transpose()
        //     .or_else(|| {
        //     self.0
        //         .flush()
        //         .map_err(Into::into)
        //         .and_then(|_| self.0.take().map_err(Into::into))
        //         .transpose()
        // })
    }
}
