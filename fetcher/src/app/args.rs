use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    /// Endpoint address
    #[clap(short, long, default_value = "http://localhost:3456/api/v1/")]
    pub endpoint: String,
    /// Stream URL (m3u8 file)
    pub m3u8: String,
}
