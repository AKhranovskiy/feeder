use rocket_db_pools::Connection;

use crate::internal::storage::Storage;
use crate::storage::playback::PlaybackCollection;
use crate::storage::streams::StreamCollection;

/// Type-safe access to collection.
pub trait StorageScheme {
    fn streams(&self) -> StreamCollection;
    fn playbacks(&self) -> PlaybackCollection;
}

impl StorageScheme for Connection<Storage> {
    fn streams(&self) -> StreamCollection {
        self.database(DATABASE_FEEDER)
            .collection(COLLECTION_STREAMS)
            .into()
    }

    fn playbacks(&self) -> PlaybackCollection {
        self.database(DATABASE_FEEDER)
            .collection(COLLECTION_PLAYBACKS)
            .into()
    }
}

impl StorageScheme for mongodb::Client {
    fn streams(&self) -> StreamCollection {
        self.database(DATABASE_FEEDER)
            .collection(COLLECTION_STREAMS)
            .into()
    }

    fn playbacks(&self) -> PlaybackCollection {
        self.database(DATABASE_FEEDER)
            .collection(COLLECTION_PLAYBACKS)
            .into()
    }
}

const DATABASE_FEEDER: &str = "feeder";
const COLLECTION_STREAMS: &str = "streams";
const COLLECTION_PLAYBACKS: &str = "playbacks";
