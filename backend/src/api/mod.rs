mod events;
mod segment;
mod segments;

use rocket::Route;

pub use events::FeederEvent;

pub fn routes() -> Vec<Route> {
    routes![
        events::events,
        segment::segment_audio,
        segments::segments,
        segments::segments_json,
        segments::upload::upload,
    ]
}
