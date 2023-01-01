use rand::rngs::SmallRng;
use rand::{thread_rng, RngCore, SeedableRng};
use thiserror::Error;

mod shuffle;
pub use shuffle::NdArrayShuffleExt;

mod shuffle_inplace;
pub use shuffle_inplace::NdArrayShuffleInplaceExt;

#[derive(Debug, Error, Copy, Clone, Eq, PartialEq)]
pub enum NdArrayShuffleError {
    #[error("Non-standard layout is not supported.")]
    NonStandardLayout,
    #[error("Invalid axis: {0}")]
    InvalidAxis(usize),
}

pub(crate) fn default_rng() -> Box<dyn RngCore> {
    Box::new(SmallRng::from_rng(&mut thread_rng()).unwrap())
}
