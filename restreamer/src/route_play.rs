use std::io::{Read, Write};
use std::time::Duration;

use analyzer::{BufferedAnalyzer, ContentKind, LabelSmoother};
use async_stream::stream;
use axum::body::StreamBody;
use axum::extract::{Query, State};
use codec::dsp::{CrossFade, CrossFadePair, ParabolicCrossFade};
use codec::{AudioFrame, CodecParams, Decoder, Encoder, FrameDuration, Resampler};
use futures::Stream;

use crate::play_params::{PlayAction, PlayParams};
use crate::terminate::Terminator;

pub async fn serve(
    Query(params): Query<PlayParams>,
    State(terminator): State<Terminator>,
) -> StreamBody<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
    stream! {
        let (mut reader, writer) = os_pipe::pipe()?;

        let handle = {
            let terminator = terminator.clone();
            std::thread::spawn(move || analyze(params, writer, terminator))
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

    let sample_audio = include_bytes!("../sample.mp3");
    let decoder = Decoder::try_from(std::io::Cursor::new(sample_audio))?;
    let mut resampler = Resampler::new(decoder.codec_params(), params);
    let mut frames = vec![];

    for frame in decoder {
        for frame in resampler.push(frame?)? {
            frames.push(frame?);
        }
    }

    Ok(frames)
}

fn analyze<W: Write>(params: PlayParams, writer: W, terminator: Terminator) -> anyhow::Result<()> {
    let action = params.action.unwrap_or(PlayAction::Passthrough);

    let input = unstreamer::Unstreamer::open(params.url)?;

    let decoder = Decoder::try_from(input)?;

    let sample_audio_frames = prepare_sample_audio(decoder.codec_params())?;

    const CROSS_FADE_DURATION: Duration = Duration::from_secs(2);

    let cf = ParabolicCrossFade::generate(
        (CROSS_FADE_DURATION.as_millis() / sample_audio_frames[0].duration().as_millis()) as usize,
    );

    eprintln!(
        "Cross-fade {:0.1}s, {} frames",
        CROSS_FADE_DURATION.as_secs_f32(),
        cf.len()
    );

    let mut encoder = Encoder::opus(decoder.codec_params(), writer)?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(5));

    let mut ad_iter = sample_audio_frames.iter().cycle();
    let mut cf_iter = cf.iter();

    for frame in decoder {
        let frame = frame?;

        let kind = analyzer.push(frame.clone())?;

        let frame = match kind {
            ContentKind::Music | ContentKind::Talk | ContentKind::Unknown => {
                ad_iter = sample_audio_frames.iter().cycle();
                cf_iter = cf.iter();
                frame
            }
            ContentKind::Advertisement => match action {
                PlayAction::Passthrough => frame,
                PlayAction::Silence => codec::silence_frame(&frame),
                PlayAction::Lang(_) => {
                    let cf: CrossFadePair = cf_iter.next().cloned().unwrap_or((0.0, 1.0).into());

                    let ad = if cf.fade_in() > 0.0 {
                        ad_iter
                            .next()
                            .cloned()
                            .map(|f| f.with_pts(frame.pts()))
                            .unwrap_or_else(|| codec::silence_frame(&frame))
                    } else {
                        codec::silence_frame(&frame)
                    };

                    &cf * (&frame, &ad)
                }
            },
        };

        encoder.push(frame)?;

        std::io::stdout().write_all(&kind.name().as_bytes()[..1])?;
        std::io::stdout().flush()?;

        if terminator.is_terminated() {
            break;
        }
    }

    encoder.flush()?;

    std::io::stdout().write_all("\nTerminating analyzer".as_bytes())?;
    std::io::stdout().flush()?;

    Ok(())
}
