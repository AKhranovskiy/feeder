use std::io::Write;
use std::time::Duration;

use kdam::{tqdm, BarExt};
use log::LevelFilter;
use stderrlog::Timestamp;

use analyzer::{BufferedAnalyzer, ContentKind, LabelSmoother};
use codec::Decoder;

const BUFFER_SIZE: usize = 2 * 1024;

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
        LabelSmoother::new(Duration::from_millis(0), Duration::from_millis(500)),
        false,
    );

    let mut buf = Vec::with_capacity(BUFFER_SIZE);

    let mut pb_frames = tqdm!(
        total = decoder.frames() as usize,
        desc = "Processed",
        unit = "f",
        force_refresh = true,
        position = 0,
        disable = false
    );

    let mut pb_ads = tqdm!(
        total = decoder.frames() as usize,
        desc = "Detected ads",
        unit = "f",
        force_refresh = true,
        position = 1,
        disable = false
    );

    for frame in decoder {
        if let Some((kind, _)) = analyzer.push(frame?)? {
            if kind == ContentKind::Advertisement {
                pb_ads.update(1);
            }

            let k = match kind {
                ContentKind::Advertisement => 'A',
                ContentKind::Music => 'M',
                ContentKind::Talk => 'T',
                ContentKind::Unknown => 'U',
            };

            buf.push(k as u8);
            if buf.len() == BUFFER_SIZE {
                std::io::stdout().write_all(&buf)?;
                buf.clear();
            }
            pb_frames.update(1);
        }
    }

    println!(
        "\nProcessed {}, ads={} / {}%",
        pb_frames.get_counter(),
        pb_ads.get_counter(),
        (pb_ads.get_counter() as f64) / (pb_frames.get_counter() as f64) * 100.0
    );
    std::io::stdout().write_all(&buf)?;

    Ok(())
}
