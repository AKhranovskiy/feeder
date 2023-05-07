use std::io::Write;
use std::time::Duration;

use kdam::{tqdm, BarExt};
use log::LevelFilter;
use stderrlog::Timestamp;

use analyzer::{BufferedAnalyzer, LabelSmoother};
use codec::Decoder;

const SIZE: usize = 10 * 1024;

fn main() -> anyhow::Result<()> {
    stderrlog::new()
        .show_module_names(false)
        .show_level(false)
        .module("analyzer::smooth")
        .module("analyzer::analyzer")
        .verbosity(LevelFilter::Debug)
        .timestamp(Timestamp::Second)
        .init()
        .unwrap();

    let file = std::env::args().nth(1).expect("Expects path");
    let input = std::fs::File::open(file).expect("Valid file path");

    let decoder = Decoder::try_from(input)?;

    let mut analyzer = BufferedAnalyzer::new(
        LabelSmoother::new(Duration::from_millis(200), Duration::from_millis(400)),
        false,
    );

    let mut buf = Vec::with_capacity(SIZE);

    let mut pb = tqdm!(
        total = decoder.frames() as usize,
        desc = "Processed",
        unit = "f",
        force_refresh = true
    );

    for frame in decoder {
        if let Some((kind, _)) = analyzer.push(frame?)? {
            let k = match kind {
                analyzer::ContentKind::Advertisement => 'A',
                analyzer::ContentKind::Music => 'M',
                analyzer::ContentKind::Talk => 'T',
                analyzer::ContentKind::Unknown => 'U',
            };
            buf.push(k as u8);
            if buf.len() == SIZE {
                std::io::stdout().write_all(&buf)?;
                buf.clear();
            }
        }
        pb.update(1);
    }

    std::io::stdout().write_all(&buf)?;

    Ok(())
}
