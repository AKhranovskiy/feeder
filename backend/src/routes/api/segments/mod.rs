use rocket::{http::Status, response::status};

pub mod delete;
pub mod metadata;
pub mod reasses;
pub mod search;
pub mod upload;

fn to_internal_server_error(error: anyhow::Error) -> status::Custom<String> {
    log::error!("Internal Server Error: {error:#}");
    status::Custom(Status::InternalServerError, error.to_string())
}
