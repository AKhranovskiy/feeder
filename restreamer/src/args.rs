use clap::{value_parser, Parser};

#[derive(Debug, Clone, Parser)]
pub struct Args {
    /// Listening port.
    #[arg(short, long, default_value_t = 15190)]
    #[arg(value_parser = value_parser!(u16).range(3000..))]
    pub port: u16,

    /// No logging.
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,

    /// No recordings.
    #[arg(long, default_value_t = false)]
    pub no_recordings: bool,

    /// Run on GCP: level=info, no-recordings=true
    #[arg(long, default_value_t = false)]
    pub gcp: bool,

    /// Smoothing behind buffer in ms.
    #[arg(long, default_value_t = 500)]
    #[arg(value_parser = value_parser!(u64).range(0..10_000))]
    pub smooth_behind: u64,

    /// Smoothing ahead buffer in ms.
    #[arg(long, default_value_t = 1_500)]
    #[arg(value_parser = value_parser!(u64).range(0..10_000))]
    pub smooth_ahead: u64,
}
