mod ad_cache;
mod ads_planner;
mod ads_provider;

use ad_cache::{AdCache, AdId};

pub use ads_planner::AdsPlanner;
pub use ads_provider::AdsProvider;

#[cfg(test)]
pub const CODEC_PARAMS: codec::CodecParams =
    codec::CodecParams::new(4, codec::SampleFormat::Flt, 1).with_samples_per_frame(4);
