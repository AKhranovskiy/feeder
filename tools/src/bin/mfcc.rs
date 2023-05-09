use std::{
    fs::{read_dir, File, FileType},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use bytemuck::cast_slice;
use clap::Parser;
use codec::{CodecParams, Decoder, Resampler, SampleFormat};
use kdam::{tqdm, BarExt};
use mfcc::calculate_mfccs;
use rand::{seq::SliceRandom, thread_rng};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

#[derive(Debug, Parser)]
struct Args {
    /// Path to a file or a directory with audio files to process.
    #[arg(required = true)]
    input: PathBuf,

    /// File to write coefficients in Bincode format
    #[arg(required = true)]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = mfcc::Config::default();

    let metadata = std::fs::metadata(&args.input)?;
    let files = if metadata.is_file() {
        vec![args.input.into()]
    } else if metadata.is_dir() {
        let mut files = read_dir(args.input)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().filter(FileType::is_file).is_some())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();

        files.partial_shuffle(&mut thread_rng(), 10_000);
        files.truncate(10_000);
        files
    } else {
        panic!("Should be a file or a directory");
    };

    let writer = Mutex::new(BufWriter::new(File::create(args.output)?));

    let pb = Mutex::new(tqdm!(
        total = files.len(),
        desc = "Processed",
        force_refresh = true
    ));

    codec::suppress_ffmpeg_log();

    files.into_par_iter().for_each(|path| {
        // decode
        let Ok(data) = samples(&path) else { return; };

        if data.is_empty() {
            // pb.lock()
            //     .unwrap()
            //     .write(format!("Empty data: {}", path.display()));
            return;
        }

        // calculte
        let mfccs = calculate_mfccs(&data, config).unwrap();
        if mfccs.is_empty() {
            // pb.lock()
            //     .unwrap()
            //     .write(format!("Empty mfccs: {}", path.display()));
            return;
        }

        // write
        {
            let mut writer = writer.lock().unwrap();
            bincode::serialize_into(&mut *writer, &mfccs).unwrap();
            writer.flush().unwrap();
        }

        // update
        {
            pb.lock().unwrap().update(1);
        }
    });

    Ok(())
}

fn samples(path: &Path) -> anyhow::Result<Vec<f32>> {
    const MFCCS_CODEC_PARAMS: CodecParams = CodecParams::new(22050, SampleFormat::S16, 1);

    let io: BufReader<File> = BufReader::new(File::open(path)?);
    let decoder = Decoder::try_from(io)?;
    let mut resampler = Resampler::new(decoder.codec_params(), MFCCS_CODEC_PARAMS);

    let mut output: Vec<i16> = vec![];

    for frame in decoder {
        for frame in resampler.push(frame?)? {
            for plane in frame?.planes().iter() {
                output.extend_from_slice(cast_slice(plane.data()));
            }
        }
    }

    Ok(output.into_iter().map(f32::from).collect())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};

    #[test]
    fn test_serde() {
        let mut output = Cursor::new(Vec::new());

        for i in 0..10 {
            bincode::serialize_into(&mut output, &vec![i as f32; i]).unwrap();
        }

        output.seek(SeekFrom::Start(0)).unwrap();

        while let Ok(data) = bincode::deserialize_from::<_, Vec<f32>>(&mut output) {
            let len = data.len();
            assert_eq!(vec![len as f32; len], data);
        }
    }
}
