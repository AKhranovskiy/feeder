use std::io::{Read, Write};
use std::time::Duration;

use async_stream::stream;
use axum::routing::get;
use axum::Router;
use axum::{
    body::StreamBody,
    extract::{Query, State},
    http::header,
    response::IntoResponse,
};
use codec::dsp::CrossFader;
use futures::Stream;

use analyzer::{BufferedAnalyzer, LabelSmoother};
use codec::{
    dsp::{LinearCrossFade, ParabolicCrossFade},
    AudioFrame, CodecParams, Decoder, Encoder, FrameDuration, Resampler,
};

mod play_params;
use play_params::{PlayAction, PlayParams};

mod mixer;
use mixer::{AdsMixer, Mixer, PassthroughMixer, SilenceMixer};

use crate::args::Args;
use crate::{
    stream_saver::{Destination, StreamSaver},
    terminate::Terminator,
};

#[derive(Clone)]
struct PlayState {
    terminator: Terminator,
    args: Args,
}

pub fn router(terminator: Terminator, args: Args) -> Router {
    Router::new()
        .route("/", get(serve))
        .with_state(PlayState { terminator, args })
}

async fn serve(
    Query(params): Query<PlayParams>,
    State(state): State<PlayState>,
) -> impl IntoResponse {
    log::info!(
        "Serve {}, action={:?}",
        params.url,
        params.action.as_ref().unwrap_or(&PlayAction::Passthrough)
    );

    let headers = [
        (header::CONTENT_TYPE, "audio/ogg"),
        (header::TRANSFER_ENCODING, "chunked"),
    ];

    (headers, get_stream(params, state))
}

fn get_stream(
    params: PlayParams,
    state: PlayState,
) -> StreamBody<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
    stream! {
        let (mut reader, writer) = os_pipe::pipe()?;

        let handle = {
            let state = state.clone();
            std::thread::spawn(move || analyze(params, writer, &state))
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

fn analyze<W: Write>(params: PlayParams, writer: W, state: &PlayState) -> anyhow::Result<()> {
    let action = params.action.unwrap_or(PlayAction::Passthrough);

    let input = unstreamer::Unstreamer::open(params.url)?;

    let decoder = Decoder::try_from(input)?;
    let codec_params = decoder.codec_params();

    let sample_audio_frames = prepare_sample_audio(codec_params)?;

    let mut encoder = Encoder::opus(codec_params, writer)?;

    let mut stream_saver = StreamSaver::new(&state.args, codec_params)?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(
        Duration::from_millis(state.args.smooth_behind),
        Duration::from_millis(state.args.smooth_ahead),
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

        if state.terminator.is_terminated() {
            break;
        }
    }

    encoder.flush()?;
    stream_saver.flush();

    std::io::stdout().write_all("\nTerminating analyzer".as_bytes())?;
    std::io::stdout().flush()?;

    Ok(())
}
