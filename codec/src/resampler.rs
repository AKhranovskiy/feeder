pub use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::codec::audio::AudioResampler;

use crate::{CodecParams, Decoder};

#[non_exhaustive]
pub struct Resampler(AudioResampler);

impl Resampler {
    #[must_use]
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
        self.0.try_push(frame).map(|()| self).map_err(Into::into)
    }
}

impl Iterator for Resampler {
    type Item = anyhow::Result<AudioFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map_err(Into::into).transpose()
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
