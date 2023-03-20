use axum::routing::get;
use axum::Router;

use crate::GlobalState;

mod collection;
use collection::VastCollection;

mod state;
use state::VastState;

mod root;

// Axum doc says it should be generic return type.
pub(crate) fn routes<S>(server: &str, state: GlobalState) -> Router<S> {
    Router::new()
        .route("/", get(root::serve))
        .with_state(VastState {
            collection: VastCollection::new(server),
            terminator: state.terminator,
        })
}
