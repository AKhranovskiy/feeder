mod events;
mod segments;

use rocket::Route;

pub use events::FeederEvent;

pub fn routes() -> Vec<Route> {
    routes![
        events::events,
        segments::segments,
        segments::segments_json,
        segments::upload::upload,
    ]
}
