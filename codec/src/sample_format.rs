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
            SampleFormat::S16 => AcSampleFormat::from_str("s16").expect("s16"),
            SampleFormat::Flt => AcSampleFormat::from_str("flt").expect("flt"),
            SampleFormat::FltPlanar => AcSampleFormat::from_str("fltp").expect("flt"),
        }
    }
}

impl From<AcSampleFormat> for SampleFormat {
    fn from(format: AcSampleFormat) -> Self {
        match format.name() {
            "s16" => SampleFormat::S16,
            "flt" => SampleFormat::Flt,
            "fltp" => SampleFormat::FltPlanar,
            x => unreachable!("Unknown format {}", x),
        }
    }
}
