use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use rayon::prelude::ParallelIterator;
use rayon::slice::ParallelSlice;

use codec::CodecParams;
use mfcc::calculate_mfccs;

#[derive(Debug, Parser)]
struct Args {
    /// Audio files to process
    #[arg(required = true)]
    file: PathBuf,

    /// File to write coefficients in Bincode format
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Number of coefficients
    #[arg(long, short, default_value_t = 39)]
    #[arg(value_parser = clap::value_parser!(u16).range(1..=39))]
    coeffs: u16,
}

const CHUNK: usize = 22050 * 1800; // 30min

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = mfcc::Config {
        num_coefficients: args.coeffs as usize,
        ..mfcc::Config::default()
    };

    println!("Loading audio file...");

    let io = BufReader::new(File::open(args.file)?);
    let params = CodecParams::new(22050, codec::SampleFormat::S16, 1);
    let data: Vec<i16> = codec::resample(io, params)?;
    let data: Vec<f32> = data.into_iter().map(f32::from).collect();

    println!("Ok. {} samples", data.len());

    let chunks = (data.len() as f32 / CHUNK as f32).ceil() as usize;

    print!("Processing {chunks} chunks ",);

    let instant = Instant::now();

    let coeffs = if chunks < 2 {
        println!("sequentially");
        calculate_mfccs(data.as_slice(), config)?
    } else {
        println!("in parallel");

        data.as_slice()
            .par_chunks(CHUNK)
            .filter_map(|chunk| match calculate_mfccs(chunk, config) {
                Ok(coeffs) => Some(coeffs),
                Err(err) => panic!("Failed to process chunk: {err:#}"),
            })
            .reduce(Vec::new, |mut acc, mut x| {
                acc.append(&mut x);
                acc
            })
    };

    println!("Processed in {:.02}s", instant.elapsed().as_secs_f32());

    if let Some(output) = args.output {
        println!("Writing output...");
        bincode::serialize_into(BufWriter::new(File::create(output)?), &coeffs)?;
    } else {
        println!("{coeffs:?}");
    }

    Ok(())
}
