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
        let mut buf = [0u8; 2048];
        let read = unstreamer.read(&mut buf)?;
        if read > 0 {
            std::io::stdout().write_all(&buf[0..read])?;
            std::io::stdout().flush()?;
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
