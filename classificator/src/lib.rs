pub mod config;
pub mod decode;
pub mod mfcc;

mod classificator;
mod networks;
pub(crate) mod data;
mod prediction;
mod score;
mod stat;
mod util;

pub use crate::classificator::Classificator;
pub use crate::classificator::Network;
pub use crate::classificator::Model;
