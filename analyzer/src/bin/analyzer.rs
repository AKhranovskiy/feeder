use std::time::Duration;

use codec::Decoder;

use analyzer::BufferedAnalyzer;
use analyzer::LabelSmoother;
use log::LevelFilter;
use stderrlog::Timestamp;

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
        LabelSmoother::new(Duration::from_millis(200), Duration::from_millis(300)),
        true,
    );

    for frame in decoder {
        if analyzer.push(frame?)?.is_some() {
            println!("");
        }
    }

    Ok(())
}
