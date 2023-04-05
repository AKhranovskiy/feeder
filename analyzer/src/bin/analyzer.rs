use std::time::Duration;

use codec::Decoder;

use analyzer::BufferedAnalyzer;
use analyzer::LabelSmoother;

fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1).expect("Expects path");
    let input = std::fs::File::open(file).expect("Valid file path");

    let decoder = Decoder::try_from(input)?;

    let mut analyzer = BufferedAnalyzer::new(LabelSmoother::new(
        Duration::from_millis(200),
        Duration::from_millis(100),
    ));

    for frame in decoder {
        let _ = analyzer.push(frame?)?;
    }

    Ok(())
}
