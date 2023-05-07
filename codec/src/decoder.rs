use std::io::Read;

use ac_ffmpeg::codec::audio::{AudioDecoder, AudioFrame};
use ac_ffmpeg::codec::{AudioCodecParameters, Decoder as AcDecoder};
use ac_ffmpeg::format::demuxer::{Demuxer, DemuxerWithStreamInfo};
use ac_ffmpeg::format::io::IO;

use crate::{CodecParams, ResamplingDecoder};

#[non_exhaustive]
pub struct Decoder<T> {
    demuxer: DemuxerWithStreamInfo<T>,
    decoder: AudioDecoder,
}

impl<R: Read> Decoder<R> {
    pub fn try_from(input: R) -> anyhow::Result<Self> {
        let io = IO::from_read_stream(input);

        let demuxer = Demuxer::builder()
            .build(io)?
            .find_stream_info(None)
            .map_err(|(_, err)| err)?;

        let decoder = AudioDecoder::from_stream(&demuxer.streams()[0])?.build()?;

        Ok(Self { demuxer, decoder })
    }
}

impl<T> Decoder<T> {
    #[must_use]
    pub fn codec_parameters(&self) -> AudioCodecParameters {
        self.demuxer.streams()[0]
            .codec_parameters()
            .as_audio_codec_parameters()
            .cloned()
            .unwrap()
    }

    #[must_use]
    pub fn codec_params(&self) -> CodecParams {
        CodecParams::from(&self.codec_parameters())
    }

    #[must_use]
    pub fn resample(self, target: CodecParams) -> ResamplingDecoder<T> {
        ResamplingDecoder::new(self, target)
    }

    #[must_use]
    pub fn frames(&self) -> u64 {
        self.demuxer.streams()[0].frames().unwrap_or_default()
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
        let packet = self.demuxer.take().map_err(Into::into).transpose();

        match packet {
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
