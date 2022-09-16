#![feature(trait_alias)] // trait Optional, optional.rs
#![feature(option_result_contains)] // IHR impl

use rocket::fs::{relative, FileServer};
use rocket_db_pools::Database;

mod fairings;
mod internal;
mod routes;
mod storage;

use internal::storage::Storage;
use internal::tera;

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Storage::init())
        .attach(tera::custom())
        .attach(fairings::stage())
        .mount("/", routes::root::routes())
        .mount("/api/v1", routes::api::routes())
        .mount("/static", FileServer::from(relative!("static")))
}
