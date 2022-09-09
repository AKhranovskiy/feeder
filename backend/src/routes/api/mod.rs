mod events;
mod playbacks;
pub mod segment;
pub mod segments;
mod streams;

use rocket::{routes, Route};

pub fn routes() -> Vec<Route> {
    routes![
        segment::analyse::analyse_file,
        segment::analyse::analyse_url,
        segment::audio::audio,
        segment::delete::delete,
        segment::metadata::metadata,
        segment::update::update,
        segments::delete::delete,
        segments::metadata::segments_json,
        segments::metadata::segments_msgpack,
        segments::reasses::reasses_content_kind,
        segments::search::search_json,
        segments::upload::upload,
        streams::delete::delete,
        streams::fetch::fetch,
        streams::get::get_all,
        streams::get::get_one,
        playbacks::get::segments_for_stream,
        playbacks::get::all_segments,
        playbacks::get::one_segment,
        playbacks::updates::updates,
    ]
}
