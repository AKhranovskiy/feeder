mod analyse;
mod classification;
pub mod codec;
pub mod documents;
mod download;
pub mod emysound;
mod guess;
mod optional;
pub mod prediction;
pub mod storage;
pub mod tera;

pub use analyse::analyse;
pub use analyse::FingerprintMatch;
pub use download::download;
pub use guess::guess_content_kind;
pub use optional::Optional;

/// Converts `uuid::Uuid` to `bson::Uuid`.
pub fn to_bson_uuid(uuid: uuid::Uuid) -> mongodb::bson::Uuid {
    mongodb::bson::Uuid::from_bytes(uuid.into_bytes())
}

/// Converts `bson::Uuid` to `uuid::Uuid`.
pub fn from_bson_uuid(uuid: mongodb::bson::Uuid) -> uuid::Uuid {
    uuid::Uuid::from_bytes(uuid.bytes())
}
