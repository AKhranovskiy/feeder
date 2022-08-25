mod events;
mod metadata_response;
pub mod segment;
pub mod segments;

use rocket::Route;

pub use events::FeederEvent;
pub use metadata_response::MetadataResponse;

pub fn routes() -> Vec<Route> {
    routes![
        events::events,
        segment::analyse::analyse_file,
        segment::analyse::analyse_url,
        segment::delete::delete,
        segment::metadata::metadata,
        segment::segment_audio,
        segment::update::update,
        segments::delete::delete,
        segments::reasses::reasses_content_kind,
        segments::search::search_json,
        segments::segments_json,
        segments::segments_msgpack,
        segments::upload::upload,
    ]
}
