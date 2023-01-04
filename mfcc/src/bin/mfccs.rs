// merge_mfccs: join several blobs

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use ndarray::Axis;

use mfcc::calculate_mfccs;

#[derive(Parser)]
struct Args {
    /// List of audio files to process.
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let files = Args::parse()
        .files
        .into_iter()
        .collect::<HashSet<PathBuf>>()
        .into_iter()
        .collect::<Vec<_>>();

    println!("Processing {} files", files.len());

    let instant = Instant::now();

    process(&files)?;

    println!("Done. {}ms", instant.elapsed().as_millis());
    Ok(())
}

#[cfg(feature = "parallel")]
fn process(files: &[PathBuf]) -> anyhow::Result<()> {
    use codec::CodecParams;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    let mfccs = {
        let processed = files
            .into_par_iter()
            .map(|path| {
                let io = std::io::BufReader::new(std::fs::File::open(path)?);
                let data = codec::resample::<_, f32>(
                    io,
                    CodecParams::new(22050, codec::SampleFormat::Flt, 1),
                )?;
                calculate_mfccs(data.as_slice(), Default::default()).map_err(anyhow::Error::from)
            })
            .inspect(|v| {
                if let Err(e) = v {
                    eprintln!("Failed to process file: {e:#}");
                };
            })
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let views = processed.iter().map(|a| a.view()).collect::<Vec<_>>();
        ndarray::concatenate(Axis(0), views.as_slice())?
    };

    println!("Storing result to 'mfccs.pickle'");
    {
        serde_pickle::to_writer(
            &mut std::io::BufWriter::new(std::fs::File::create("./mfccs.pickle")?),
            &mfccs,
            Default::default(),
        )?;
    }

    Ok(())
}

#[cfg(feature = "async")]
fn process(files: &[PathBuf]) -> anyhow::Result<()> {
    // I use tokio::fs, so it must be Tokio runtime.
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .unwrap()
        .block_on(async move { async_process(files).await })
        .map_err(Into::into)
}

#[cfg(feature = "async")]
async fn async_process(files: &[PathBuf]) -> anyhow::Result<()> {
    use futures::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};

    futures::stream::iter(files)
        .then(tokio::fs::read)
        .inspect_err(|err| {
            eprintln!("Failed to load file: {err:#}");
        })
        .map(|v| v.map_err(Into::into))
        .and_then(|buf| async move { codec::decode(std::io::Cursor::new(buf)) })
        .inspect_err(|err| {
            eprintln!("Failed to decode audio file: {err:#}");
        })
        .and_then(|v| async move {
            let v = v.into_iter().map(f32::from).collect::<Vec<_>>();
            calculate_mfccs(&v, Default::default())
        })
        .inspect_err(|err| {
            eprintln!("Failed to calculate MFCCs: {err:#}");
        })
        .filter_map(|v| async move { v.ok() })
        .collect::<Vec<_>>()
        .then(|v| async move {
            let views = v.iter().map(|a| a.view()).collect::<Vec<_>>();
            ndarray::concatenate(Axis(0), views.as_slice()).map_err(anyhow::Error::from)
        })
        .and_then(
            |v| async move { serde_pickle::to_vec(&v, Default::default()).map_err(Into::into) },
        )
        .and_then(|v| tokio::fs::write("./mfccs.pickle", v).map_err(Into::into))
        .await?;
    Ok(())
}
