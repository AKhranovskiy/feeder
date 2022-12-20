use std::io::{Read, Write};

use clap::Parser;
use unstreamer::Unstreamer;
use url::Url;

#[derive(Parser)]
struct Args {
    source: Url,
}

fn main() -> anyhow::Result<()> {
    let mut unstreamer = Unstreamer::open(Args::parse().source)?;
    loop {
        let mut buf = vec![];
        if unstreamer.read_to_end(&mut buf)? > 0 {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&buf)?;
            stdout.flush()?;
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
