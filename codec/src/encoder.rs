use std::io::Write;

use ac_ffmpeg::codec::audio::AudioEncoder;
use ac_ffmpeg::codec::audio::AudioFrame;
use ac_ffmpeg::codec::Encoder as AsEncoder;
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::format::muxer::Muxer;
use ac_ffmpeg::format::muxer::OutputFormat;

use crate::CodecParams;
use crate::Resampler;
use crate::SampleFormat;

static OPUS_SAMPLE_RATE: u32 = 48_000;
static OPUS_SAMPLE_FORMAT: SampleFormat = SampleFormat::Flt;

#[non_exhaustive]
pub struct Encoder<T> {
    encoder: AudioEncoder,
    muxer: Muxer<T>,
    resampler: Resampler,
}

impl<W: Write> Encoder<W> {
    pub fn opus(params: CodecParams, output: W) -> anyhow::Result<Self> {
        let encoder = AudioEncoder::builder("libopus")?
            .sample_rate(OPUS_SAMPLE_RATE)
            .sample_format(OPUS_SAMPLE_FORMAT.into())
            .bit_rate(params.bit_rate)
            .channel_layout(params.channel_layout())
            .build()?;

        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&encoder.codec_parameters().into())?;

        let muxer = muxer_builder.build(
            IO::from_write_stream(output),
            OutputFormat::find_by_name("ogg").expect("output format"),
        )?;

        let target = {
            let mut params = CodecParams::from(&encoder.codec_parameters());
            params.samples_per_frame = encoder.samples_per_frame();
            params
        };

        let resampler = Resampler::new(params, target);

        Ok(Self {
            encoder,
            muxer,
            resampler,
        })
    }

    pub fn push(&mut self, frame: AudioFrame) -> anyhow::Result<&mut Self> {
        for frame in self.resampler.push(frame)? {
            self.encoder.try_push(frame?)?;
            while let Some(packet) = self.encoder.take()? {
                self.muxer.push(packet)?;
            }
        }

        Ok(self)
    }

    pub fn flush(&mut self) -> anyhow::Result<&mut Self> {
        self.encoder.try_flush()?;

        while let Some(packet) = self.encoder.take()? {
            self.muxer.push(packet)?;
        }

        self.muxer.flush()?;

        Ok(self)
    }
}
