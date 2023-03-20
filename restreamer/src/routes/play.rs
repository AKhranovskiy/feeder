use std::io::{Read, Write};
use std::iter::repeat;
use std::time::Duration;

use analyzer::{BufferedAnalyzer, ContentKind, LabelSmoother};
use async_stream::stream;
use axum::body::StreamBody;
use axum::extract::{Query, State};
use codec::dsp::{CrossFade, CrossFadePair, LinearCrossFade, ParabolicCrossFade, ToFadeInOut};
use codec::{AudioFrame, CodecParams, Decoder, Encoder, FrameDuration, Resampler};
use futures::Stream;

use crate::terminate::Terminator;
use crate::GlobalState;

mod mixer;
mod params;
mod recorder;

use mixer::{AdsMixer, Mixer, PassthroughMixer, SilenceMixer};
use params::{PlayAction, PlayParams};
use recorder::{Destination, Recorder};

pub(crate) async fn serve(
    Query(params): Query<PlayParams>,
    State(state): State<GlobalState>,
) -> StreamBody<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
    stream! {
        let (mut reader, writer) = os_pipe::pipe()?;

        let handle = {
            let terminator = state.terminator.clone();
            std::thread::spawn(move || analyze(params, writer, &terminator))
        };

        let mut buf = [0u8;1024];

        loop {
            if handle.is_finished() {
                handle.join().unwrap()?;
                break;
            }
            if state.terminator.is_terminated() {
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

    let mut recorder = Recorder::new(decoder.codec_params())?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(
        Duration::from_millis(1500),
        Duration::from_millis(1000),
    ));

    let mut mixer: Box<dyn Mixer> = match action {
        PlayAction::Passthrough => Box::new(PassthroughMixer::new()),
        PlayAction::Silence => Box::new(SilenceMixer::new(&cf)),
        PlayAction::Lang(_) => Box::new(AdsMixer::new(&sample_audio_frames, &cf)),
    };

    let entry_fade_in = LinearCrossFade::generate(cf.len()).to_fade_in();
    let mut efi_iter = entry_fade_in.iter().chain(repeat(&CrossFadePair::END));

    for frame in decoder {
        let frame = frame?;
        let kind = analyzer.push(frame.clone())?;

        recorder.push(Destination::Original, frame.clone());

        let frame = match kind {
            ContentKind::Advertisement => mixer.advertisement(&frame),
            ContentKind::Music | ContentKind::Talk | ContentKind::Unknown => mixer.content(&frame),
        };

        let frame = efi_iter.next().unwrap() * (&frame, &frame);

        recorder.push(Destination::Processed, frame.clone());

        encoder.push(frame)?;

        if terminator.is_terminated() {
            break;
        }
    }

    encoder.flush()?;
    recorder.flush();

    std::io::stdout().write_all("\nTerminating analyzer".as_bytes())?;
    std::io::stdout().flush()?;

    Ok(())
}
