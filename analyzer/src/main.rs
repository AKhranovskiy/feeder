use std::{collections::VecDeque, io::Write, str::FromStr};

use ac_ffmpeg::{
    codec::{
        audio::{
            AudioDecoder, AudioEncoder, AudioFrame, AudioResampler, ChannelLayout, SampleFormat,
        },
        Decoder, Encoder,
    },
    format::{
        demuxer::Demuxer,
        io::IO,
        muxer::{Muxer, OutputFormat},
        stream::Stream,
    },
};
use anyhow::anyhow;
use bytemuck::cast_slice;
use ndarray_stats::QuantileExt;

use classifier::Classifier;

mod smooth;
use smooth::LabelSmoother;

/**
 * Takes stdin, decode, analyze, put encoded/muxed stream to stdin and analysis result to stderr.
 */
fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let io = IO::from_read_stream(stdin);

    let mut demuxer = Demuxer::builder()
        .build(io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)?;

    let mut decoder = AudioDecoder::from_stream(&demuxer.streams()[0])?.build()?;

    let codec = demuxer.streams()[0].codec_parameters();
    let params = codec.as_audio_codec_parameters().unwrap();

    let sample_format_flt = SampleFormat::from_str("flt").expect("Sample format FLT");
    let opus_sample_rate = 48000;

    let mut encoder = AudioEncoder::builder("libopus")?
        .sample_rate(opus_sample_rate)
        .bit_rate(params.bit_rate())
        .sample_format(sample_format_flt)
        .channel_layout(params.channel_layout().to_owned())
        .build()?;

    let mut resampler = AudioResampler::builder()
        .source_sample_rate(params.sample_rate())
        .source_sample_format(params.sample_format())
        .source_channel_layout(params.channel_layout())
        .target_frame_samples(encoder.samples_per_frame())
        .target_sample_rate(opus_sample_rate)
        .target_sample_format(sample_format_flt)
        .target_channel_layout(params.channel_layout())
        .build()?;

    let mut muxer = {
        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&encoder.codec_parameters().into())?;
        muxer_builder.build(
            IO::from_write_stream(std::io::stdout()),
            OutputFormat::find_by_name("ogg").expect("output format"),
        )?
    };

    let mut analyzer = BufferedAnalyzer::new();

    while let Some(packet) = demuxer.take()? {
        decoder.push(packet)?;

        while let Some(frame) = decoder.take()? {
            if let Some(class) = analyzer.push(frame.clone())? {
                std::io::stderr().write_all(class.as_bytes())?;
            }

            resampler.push(frame)?;

            while let Some(frame) = resampler.take()? {
                encoder.push(frame)?;

                while let Some(encoded_frame) = encoder.take()? {
                    muxer.push(encoded_frame)?;
                }
            }
        }
        muxer.flush()?;
    }

    Ok(())
}

fn _print_info(stream: &Stream) {
    let codec = stream.codec_parameters();
    let codec = codec.as_audio_codec_parameters().unwrap();

    println!(
        r#"
        Stream info
            Start time: {:?}
            Duration: {:?}
            Frames: {:?}

            Codec
                decoder name: {}
                bit rate: {}kbps
                sample rate: {}Hz
                sample format: {}
                channels: {:?}
            "#,
        stream.start_time(),
        stream.duration(),
        stream.frames(),
        codec.decoder_name().unwrap_or("unknown"),
        codec.bit_rate() / 1_000,
        codec.sample_rate(),
        codec.sample_format().name(),
        codec.channel_layout().channels()
    );
}

struct BufferedAnalyzer {
    queue: VecDeque<f64>,
    classifer: Classifier,
    smoother: LabelSmoother,
}

impl BufferedAnalyzer {
    fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(150 * 39 * 2),
            classifer: Classifier::from_file("./model").expect("Initialized classifier"),
            smoother: LabelSmoother::new(10),
        }
    }

    fn resample(frame: AudioFrame) -> anyhow::Result<AudioFrame> {
        let mut resampler = AudioResampler::builder()
            .source_sample_rate(frame.sample_rate())
            .source_sample_format(frame.sample_format())
            .source_channel_layout(frame.channel_layout())
            .target_sample_rate(22050)
            .target_sample_format(
                SampleFormat::from_str("s16").expect("Sample format for analysis"),
            )
            .target_channel_layout(ChannelLayout::from_channels(1).expect("Mono channel layout"))
            .build()?;

        resampler.push(frame)?;
        resampler
            .take()?
            .ok_or_else(|| anyhow!("Resampler returns no data"))
    }

    fn push(&mut self, frame: AudioFrame) -> anyhow::Result<Option<&'static str>> {
        if frame.samples() < 128 {
            return Ok(None);
        }

        let frame = Self::resample(frame)?;

        let data = cast_slice::<u8, i16>(frame.planes()[0].data())
            .iter()
            .cloned()
            .map(f32::from)
            .collect::<Vec<_>>();

        let coeffs = mfcc::Config::default().num_coefficients;

        let mut mfccs = mfcc::calculate_mfccs(&data, Default::default())?
            .into_iter()
            .map(f64::from)
            .collect::<VecDeque<_>>();

        self.queue.append(&mut mfccs);

        if self.queue.len() >= (150 * coeffs) {
            let data = self
                .queue
                .iter()
                .take(150 * coeffs)
                .cloned()
                .collect::<Vec<_>>();

            let data = ndarray::Array4::from_shape_vec((1, 150, coeffs, 1), data)?;

            self.queue.drain(..(100 * coeffs));

            let prediction = self.classifer.predict(&data)?;
            let prediction = self.smoother.push(prediction);

            match prediction.argmax()?.1 {
                0 => Ok(Some("A")),
                1 => Ok(Some("M")),
                2 => Ok(Some("T")),
                _ => unreachable!("Unexpected prediction shape"),
            }
        } else {
            Ok(None)
        }
    }
}
