mod events;
mod segments;

use rocket::Route;

pub use events::FeederEvent;

pub fn routes() -> Vec<Route> {
    routes![segments::upload::upload, events::events]
}
