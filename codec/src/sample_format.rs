use std::str::FromStr;

use ac_ffmpeg::codec::audio::SampleFormat as AcSampleFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    S16,
    Flt,
    FltPlanar,
}

impl From<SampleFormat> for AcSampleFormat {
    fn from(format: SampleFormat) -> Self {
        match format {
            SampleFormat::S16 => Self::from_str("s16").expect("s16"),
            SampleFormat::Flt => Self::from_str("flt").expect("flt"),
            SampleFormat::FltPlanar => Self::from_str("fltp").expect("flt"),
        }
    }
}

impl From<AcSampleFormat> for SampleFormat {
    fn from(format: AcSampleFormat) -> Self {
        match format.name() {
            "s16" => Self::S16,
            "flt" => Self::Flt,
            "fltp" => Self::FltPlanar,
            x => unreachable!("Unknown format {}", x),
        }
    }
}
