use std::{
    io::{Read, Write},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
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
    Decoder, Encoder, FrameDuration,
};

mod play_params;
use play_params::{PlayAction, PlayParams};

mod mixer;
use mixer::{AdsMixer, Mixer, PassthroughMixer, SilenceMixer};

use crate::{
    accept_header::Accept,
    ads_planner::AdsPlanner,
    ads_provider::AdsProvider,
    args::Args,
    stream_saver::{Destination, StreamSaver},
    terminate::Terminator,
};

const OUTPUT_MIME: &str = "audio/aac";

#[derive(Clone)]
struct PlayState {
    terminator: Terminator,
    ads_provider: Arc<AdsProvider>,
    args: Args,
}

pub fn router(terminator: Terminator, ads_provider: Arc<AdsProvider>, args: Args) -> Router {
    Router::new().route("/", get(serve)).with_state(PlayState {
        terminator,
        ads_provider,
        args,
    })
}

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
        let (mut reader, writer) = match os_pipe::pipe() {
            Ok((r,w)) => (r,w),
            Err(err) => {
                log::error!("Error: failed to open pipe, {err:?}");
                Err(err)?
            },
        };

        let handle = {
            let state= state.clone();
            std::thread::spawn(move || {
                tokio::runtime::Runtime::new()?.block_on(async move {
                    analyze(params, writer, &state).await
                })})
        };

        let mut buf = [0u8;1024];

        // let mut rate = Rate::new();

        loop {
            if handle.is_finished() {
                if let Err(err) = handle.join().unwrap(){
                    log::error!("Analyzer failed: {err:?}");
                    Err(err)?;
                }
                break;
            }
            if state.terminator.is_terminated() {
                break;
            }

            let read = match reader.read(&mut buf){
                Ok(read) =>read,
                Err(err) => {
                    log::error!("Reader failed: {err:?}");
                    Err(err)?
                },
            };

            // let r = rate.push(read) / 128;
            // print!("\r{r} kbps");

            yield Ok(buf[0..read].to_vec())
        }
    }
    .into()
}

const CROSS_FADE_DURATION: Duration = Duration::from_millis(1_500);

async fn analyze<W: Write + Send>(
    params: PlayParams,
    writer: W,
    state: &PlayState,
) -> anyhow::Result<()> {
    let input = unstreamer::Unstreamer::open(&params.source)?;

    let mut decoder = Decoder::try_from(input)?;
    let first_frame = decoder.next().ok_or_else(|| anyhow!("No audio frame"))??;

    let codec_params = decoder
        .codec_params()
        .with_samples_per_frame(first_frame.samples());

    log::info!("Input media info {codec_params:?}");

    let mut encoder = Encoder::aac(codec_params, writer)?;
    log::info!("Output media info {:?}", encoder.codec_params());

    let mut stream_saver = StreamSaver::new(state.args.is_recording_enabled(), codec_params)?;

    let mut analyzer = BufferedAnalyzer::new(
        LabelSmoother::new(
            Duration::from_millis(state.args.smooth_behind),
            Duration::from_millis(state.args.smooth_ahead),
        ),
        state.args.clone().into(),
    );

    let cross_fader =
        CrossFader::new::<ParabolicCrossFade>(CROSS_FADE_DURATION, first_frame.duration());

    let entry = CrossFader::new::<LinearCrossFade>(CROSS_FADE_DURATION, first_frame.duration());

    let action = params.action.unwrap_or(PlayAction::Passthrough);
    let mut mixer: Box<dyn Mixer> = match action {
        PlayAction::Passthrough => Box::new(PassthroughMixer::new()),
        PlayAction::Silence => Box::new(SilenceMixer::new(cross_fader)),
        PlayAction::Replace => Box::new(AdsMixer::new(
            AdsPlanner::new(state.ads_provider.clone(), codec_params).await?,
            encoder.pts()?,
            cross_fader,
        )),
    };

    for frame in decoder {
        if state.terminator.is_terminated() {
            break;
        }

        let frame = frame?;
        stream_saver.push(Destination::Original, frame.clone());

        analyzer.push(frame)?;

        for (kind, frame) in analyzer.pop()? {
            if state.terminator.is_terminated() {
                break;
            }

            let frame = mixer.push(kind, &frame).await;
            let frame = entry.apply(&codec::silence_frame(&frame), &frame);

            stream_saver.push(Destination::Processed, frame.clone());

            encoder.push(frame)?;
        }
    }

    stream_saver.terminate();

    encoder.flush()?;

    log::info!("Terminating analyzer");

    Ok(())
}
