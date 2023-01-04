use std::io::Read;

use ac_ffmpeg::codec::audio::{AudioDecoder, AudioFrame};
use ac_ffmpeg::codec::{AudioCodecParameters, Decoder as AcDecoder};

use crate::resampler::{CodecParams, ResamplingDecoder};
use crate::Demuxer;

#[non_exhaustive]
pub struct Decoder<T> {
    demuxer: Demuxer<T>,
    decoder: AudioDecoder,
}

impl<R: Read> Decoder<R> {
    pub fn try_from(input: R) -> anyhow::Result<Self> {
        Demuxer::try_from(input).and_then(TryInto::try_into)
    }
}
impl<T> Decoder<T> {
    pub fn codec_parameters(&self) -> AudioCodecParameters {
        self.demuxer
            .stream()
            .codec_parameters()
            .as_audio_codec_parameters()
            .cloned()
            .unwrap()
    }

    pub fn resample(self, target: CodecParams) -> ResamplingDecoder<T> {
        ResamplingDecoder::new(self, target)
    }
}

impl<T> TryFrom<Demuxer<T>> for Decoder<T> {
    type Error = anyhow::Error;

    fn try_from(demuxer: Demuxer<T>) -> Result<Self, Self::Error> {
        let decoder = AudioDecoder::from_stream(demuxer.stream())?.build()?;
        Ok(Self { demuxer, decoder })
    }
}

impl<T> Iterator for Decoder<T> {
    type Item = anyhow::Result<AudioFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        // Is there anything in decoder already?
        let frame = self.decoder.take().map_err(Into::into).transpose();
        if frame.is_some() {
            return frame;
        }

        // If not, push demuxed packet
        match self.demuxer.next() {
            None => {}
            Some(Ok(packet)) => {
                if let Err(error) = self.decoder.try_push(packet) {
                    return Some(Err(error.into()));
                }
                return self.decoder.take().map_err(Into::into).transpose();
            }
            Some(Err(error)) => return Some(Err(error)),
        }

        // If no packet, flush decoder.
        if let Err(error) = self.decoder.flush() {
            return Some(Err(error.into()));
        }

        self.decoder.take().map_err(Into::into).transpose()
    }
}