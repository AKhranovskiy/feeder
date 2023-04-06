use std::io::{Read, Write};
use std::time::Duration;

use async_stream::stream;
use axum::{
    body::StreamBody,
    extract::{Query, State},
};
use codec::dsp::CrossFader;
use futures::Stream;

use analyzer::{BufferedAnalyzer, LabelSmoother};
use codec::{
    dsp::{CrossFade, LinearCrossFade, ParabolicCrossFade},
    AudioFrame, CodecParams, Decoder, Encoder, FrameDuration, Resampler,
};

mod play_params;
use play_params::{PlayAction, PlayParams};

mod mixer;
use mixer::{AdsMixer, Mixer, PassthroughMixer, SilenceMixer};

use crate::{
    stream_saver::{Destination, StreamSaver},
    terminate::Terminator,
};

pub async fn serve(
    Query(params): Query<PlayParams>,
    State(terminator): State<Terminator>,
) -> StreamBody<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
    stream! {
        let (mut reader, writer) = os_pipe::pipe()?;

        let handle = {
            let terminator = terminator.clone();
            std::thread::spawn(move || analyze(params, writer, &terminator))
        };

        let mut buf = [0u8;1024];

        loop {
            if handle.is_finished() {
                handle.join().unwrap()?;
                break;
            }
            if terminator.is_terminated() {
                break;
            }

            let read = reader.read(&mut buf)?;
            yield Ok(buf[0..read].to_vec())
        }
    }
    .into()
}

pub fn prepare_sample_audio(params: CodecParams) -> anyhow::Result<Vec<AudioFrame>> {
    // TODO get samples per frame by other mean
    // TODO resample() does not work
    let params = params.with_samples_per_frame(2048); // for OGG

    let sample_audio = include_bytes!("../../sample.mp3");
    let decoder = Decoder::try_from(std::io::Cursor::new(sample_audio))?;
    let mut resampler = Resampler::new(decoder.codec_params(), params);
    let mut frames = vec![];

    for frame in decoder {
        for frame in resampler.push(frame?)? {
            frames.push(frame?);
        }
    }

    frames.truncate(100);
    Ok(frames)
}

const CROSS_FADE_DURATION: Duration = Duration::from_millis(1_500);

fn analyze<W: Write>(params: PlayParams, writer: W, terminator: &Terminator) -> anyhow::Result<()> {
    let action = params.action.unwrap_or(PlayAction::Passthrough);

    let input = unstreamer::Unstreamer::open(params.url)?;

    let decoder = Decoder::try_from(input)?;

    let sample_audio_frames = prepare_sample_audio(decoder.codec_params())?;

    let cf = ParabolicCrossFade::generate(
        (CROSS_FADE_DURATION.as_millis() / sample_audio_frames[0].duration().as_millis()) as usize,
    );
    eprintln!(
        "Cross-fade {:0.1}s, {} frames",
        CROSS_FADE_DURATION.as_secs_f32(),
        cf.len()
    );

    let mut encoder = Encoder::opus(decoder.codec_params(), writer)?;

    let mut stream_saver = StreamSaver::new(decoder.codec_params())?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(
        Duration::from_millis(300),
        Duration::from_millis(100),
    ));

    let cross_fader = CrossFader::new::<ParabolicCrossFade>(
        CROSS_FADE_DURATION,
        sample_audio_frames[0].duration(),
    );

    let entry =
        CrossFader::new::<LinearCrossFade>(CROSS_FADE_DURATION, sample_audio_frames[0].duration());

    let mut mixer: Box<dyn Mixer> = match action {
        PlayAction::Passthrough => Box::new(PassthroughMixer::new()),
        PlayAction::Silence => Box::new(SilenceMixer::new(cross_fader)),
        PlayAction::Lang(_) => Box::new(AdsMixer::new(sample_audio_frames, cross_fader)),
    };


    for frame in decoder {
        let frame = frame?;
        let kind = analyzer.push(frame.clone())?;

        stream_saver.push(Destination::Original, frame.clone());

        let frame = mixer.push(kind, &frame);
        let frame = entry.apply(&frame, &frame);

        stream_saver.push(Destination::Processed, frame.clone());

        encoder.push(frame)?;

        if terminator.is_terminated() {
            break;
        }
    }

    encoder.flush()?;
    stream_saver.flush();

    std::io::stdout().write_all("\nTerminating analyzer".as_bytes())?;
    std::io::stdout().flush()?;

    Ok(())
}
