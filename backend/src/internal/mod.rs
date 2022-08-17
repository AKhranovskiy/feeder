mod analyse;
mod classification;
pub mod codec;
pub mod documents;
pub mod emysound;
pub mod prediction;
pub mod storage;
mod tags;

pub use analyse::analyse;
pub use analyse::FingerprintMatch;
pub use tags::guess_content_kind;
