use crate::TagAnalyser;

mod adcontext;
mod ihr;
mod regular_track;
mod spotblock;
mod ttwn;

pub const TAG_ANALYSERS: &[&dyn TagAnalyser] = &[
    &adcontext::AdContext,
    &regular_track::RegularTrack,
    &ttwn::Ttwn,
    &spotblock::SpotBlock,
    &ihr::IhrPromo,
    &ihr::IHeartRadio,
];
