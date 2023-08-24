use analyzer::AnalyzerOpts;
use clap::{value_parser, Parser};
use enumflags2::BitFlags;

#[derive(Debug, Clone, Parser)]
#[allow(clippy::struct_excessive_bools)]
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

    /// Run on GCP: level=info, no-recordings=true, buffer_stat=false
    #[arg(long, default_value_t = false)]
    pub gcp: bool,

    /// Smoothing behind buffer in ms.
    #[arg(long, default_value_t = 0)]
    #[arg(value_parser = value_parser!(u64).range(0..10_000))]
    pub smooth_behind: u64,

    /// Smoothing ahead buffer in ms.
    #[arg(long, default_value_t = 1500)]
    #[arg(value_parser = value_parser!(u64).range(0..10_000))]
    pub smooth_ahead: u64,

    #[arg(long, default_value_t = false)]
    pub buffer_stat: bool,

    /// Print Error if frame processing time exceeds frame duration.
    #[arg(long, default_value_t = false)]
    pub report_slow_processing: bool,
}

impl Args {
    pub const fn is_recording_enabled(&self) -> bool {
        !self.gcp && !self.no_recordings
    }

    pub const fn buffer_stat(&self) -> bool {
        !self.gcp && !self.quiet && self.buffer_stat
    }

    pub const fn report_slow_processing(&self) -> bool {
        !self.gcp && !self.quiet && self.report_slow_processing
    }
}

impl From<Args> for BitFlags<AnalyzerOpts> {
    fn from(args: Args) -> Self {
        let mut opts = BitFlags::empty();
        if args.buffer_stat() {
            opts.insert(AnalyzerOpts::ShowBufferStatistic);
        }
        if args.report_slow_processing() {
            opts.insert(AnalyzerOpts::ReportSlowProcessing);
        }
        opts
    }
}
