use std::path::PathBuf;

use clap::Parser;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde_pickle::from_reader;

#[derive(Parser)]
struct Args {
    /// List of MFCCs Pickle files to merge. Order is important.
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let files = Args::parse().files;

    println!("Processing {} files", files.len());

    let arrays = files
        .into_par_iter()
        .map(|path| {
            from_reader(std::fs::File::open(path)?, Default::default()).map_err(anyhow::Error::from)
        })
        .collect::<anyhow::Result<Vec<ndarray::Array2<f32>>>>()?;

    serde_pickle::to_writer(
        &mut std::fs::File::create("./merged.pickle")?,
        &arrays,
        Default::default(),
    )?;

    Ok(())
}
