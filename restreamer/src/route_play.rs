use std::io::{Read, Write};

use analyzer::{BufferedAnalyzer, ContentKind, LabelSmoother};
use async_stream::stream;
use axum::body::StreamBody;
use axum::extract::{Query, State};
use codec::{AudioFrame, CodecParams, Decoder, Encoder, Resampler};
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

    let mut encoder = Encoder::opus(decoder.codec_params(), writer)?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(5));

    let mut iter = sample_audio_frames.iter().cycle();

    for frame in decoder {
        let frame = frame?;

        let kind = analyzer.push(frame.clone())?;

        let frame = match kind {
            ContentKind::Music | ContentKind::Talk | ContentKind::Unknown => {
                iter = sample_audio_frames.iter().cycle();
                frame
            }
            ContentKind::Advertisement => match action {
                PlayAction::Passthrough => frame,
                PlayAction::Silence => codec::silence_frame(&frame),
                PlayAction::Lang(_) => iter
                    .next()
                    .cloned()
                    .map(|f| f.with_pts(frame.pts()))
                    .unwrap_or_else(|| codec::silence_frame(&frame)),
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
