use codec::resample_16k_mono_s16_stream;

fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Expects audio file");
    let io = std::io::BufReader::new(std::fs::File::open(input)?);

    let output: Vec<i16> = resample_16k_mono_s16_stream(io)?;

    let norm = normalize(output);

    eprintln!("{} samples, {:?}", norm.len(), &norm[0..100]);

    Ok(())
}

fn normalize(samples: Vec<i16>) -> Vec<f32> {
    samples
        .into_iter()
        .map(|x| f32::from(x) / 32768.0)
        .collect()
}
