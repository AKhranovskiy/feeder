use axum::routing::get;
use axum::Router;

use crate::terminate::Terminator;

mod collection;
use collection::VastCollection;

mod state;
use state::VastState;

mod root;

// Axum doc says it should be generic return type.
pub fn routes<S>(server: &str, terminator: Terminator) -> Router<S> {
    Router::new()
        .route("/", get(root::serve))
        .with_state(VastState {
            collection: VastCollection::new(server),
            terminator,
        })
}
