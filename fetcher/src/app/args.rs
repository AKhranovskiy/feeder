use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    /// Stream URL (m3u8 file)
    pub m3u8: String,
}
