use rocket::{routes, Route};

mod analyse;
mod index;
mod search;
mod streams;
mod view;

pub fn routes() -> Vec<Route> {
    routes![
        analyse::analyse_get,
        analyse::analyse_post,
        index::index,
        search::search,
        view::view,
        streams::streams,
    ]
}
