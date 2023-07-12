use std::io::{Read, Write};
use std::time::Duration;

use async_stream::stream;
use axum::{
    body::StreamBody,
    extract::{Query, State},
    http::header::{self},
    response::IntoResponse,
    routing::get,
    Router, TypedHeader,
};
use futures::Stream;

use analyzer::{BufferedAnalyzer, LabelSmoother};
use codec::{
    dsp::{CrossFader, LinearCrossFade, ParabolicCrossFade},
    AudioFrame, CodecParams, Decoder, Encoder, FrameDuration, Resampler,
};

mod play_params;
use play_params::{PlayAction, PlayParams};

mod mixer;
use mixer::{AdsMixer, Mixer, PassthroughMixer, SilenceMixer};

use crate::accept_header::Accept;
use crate::args::Args;
use crate::{
    stream_saver::{Destination, StreamSaver},
    terminate::Terminator,
};

const OUTPUT_MIME: &str = "audio/aac";

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

#[allow(clippy::unused_async)]
async fn serve(
    TypedHeader(accept): TypedHeader<Accept>,
    Query(params): Query<PlayParams>,
    State(state): State<PlayState>,
) -> impl IntoResponse {
    log::info!(
        "Serve {}, action={:?}",
        params.source,
        params.action.as_ref().unwrap_or(&PlayAction::Passthrough)
    );

    log::info!("Client accepts: {accept}");
    log::info!("Server serves: {OUTPUT_MIME}");

    let headers = [
        (header::CONTENT_TYPE, OUTPUT_MIME),
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
    let sample_audio = include_bytes!("../../sample.aac");
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

    let input = unstreamer::Unstreamer::open(&params.source)?;

    let decoder = Decoder::try_from(input)?;
    let codec_params = decoder.codec_params();
    log::info!("Input media info {codec_params:?}");

    let mut encoder = Encoder::aac(codec_params, writer)?;
    log::info!("Output media info {:?}", encoder.codec_params());

    let sample_audio_frames = prepare_sample_audio(codec_params)?;

    let mut stream_saver = StreamSaver::new(state.args.is_recording_enabled(), codec_params)?;

    let mut analyzer = BufferedAnalyzer::new(
        LabelSmoother::new(
            Duration::from_millis(state.args.smooth_behind),
            Duration::from_millis(state.args.smooth_ahead),
        ),
        state.args.clone().into(),
    );

    let cross_fader = CrossFader::new::<ParabolicCrossFade>(
        CROSS_FADE_DURATION,
        sample_audio_frames[0].duration(),
    );

    let entry =
        CrossFader::new::<LinearCrossFade>(CROSS_FADE_DURATION, sample_audio_frames[0].duration());

    let mut mixer: Box<dyn Mixer> = match action {
        PlayAction::Passthrough => Box::new(PassthroughMixer::new()),
        PlayAction::Silence => Box::new(SilenceMixer::new(cross_fader)),
        PlayAction::Replace => Box::new(AdsMixer::new(sample_audio_frames, cross_fader)),
    };

    for frame in decoder {
        let frame = frame?;
        if let Some((kind, frame)) = analyzer.push(frame.clone())? {
            stream_saver.push(Destination::Original, frame.clone());

            let frame = mixer.push(kind, &frame);
            let frame = entry.apply(&codec::silence_frame(&frame), &frame);
            encoder.push(frame.clone())?;

            stream_saver.push(Destination::Processed, frame);
        }

        if state.terminator.is_terminated() {
            break;
        }
    }

    encoder.flush()?;

    stream_saver.terminate();

    log::info!("Terminating analyzer");

    Ok(())
}
