mod events;
mod metadata_response;
mod segment;
mod segments;

use rocket::Route;

pub use events::FeederEvent;
pub use metadata_response::MetadataResponse;

pub fn routes() -> Vec<Route> {
    routes![
        events::events,
        segment::metadata::metadata,
        segment::update::update,
        segment::segment_audio,
        segments::segments_json,
        segments::segments_msgpack,
        segments::upload::upload,
    ]
}
