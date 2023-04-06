mod cross_fader;
mod crossfade;

pub use crossfade::{
    CossinCrossFade, CrossFade, CrossFadePair, EqualPowerCrossFade, LinearCrossFade,
    ParabolicCrossFade, SemicircleCrossFade, ToFadeInOut,
};

pub use cross_fader::CrossFader;
