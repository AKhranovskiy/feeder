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

/// Converts `uuid::Uuid` to `bson::Uuid`.
pub fn to_bson_uuid(uuid: uuid::Uuid) -> mongodb::bson::Uuid {
    mongodb::bson::Uuid::from_bytes(uuid.into_bytes())
}

/// Converts `bson::Uuid` to `uuid::Uuid`.
pub fn from_bson_uuid(uuid: mongodb::bson::Uuid) -> uuid::Uuid {
    uuid::Uuid::from_bytes(uuid.bytes())
}
