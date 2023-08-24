#![allow(non_snake_case)]

use enumflags2::bitflags;

mod amplify;
mod analyzer;
mod content_kind;
mod rate;
mod smooth;

pub use analyzer::BufferedAnalyzer;
pub use content_kind::ContentKind;
pub use smooth::LabelSmoother;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AnalyzerOpts {
    ShowBufferStatistic,
    ReportSlowProcessing,
}
