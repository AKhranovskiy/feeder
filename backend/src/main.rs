#![feature(decl_macro)]

#[macro_use]
extern crate rocket;

use api::FeederEvent;
use rocket::fs::{relative, FileServer};
use rocket::tokio::sync::broadcast::channel;
use rocket::Config;
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};

mod api;
pub mod internal;

#[launch]
fn rocket() -> _ {
    let config = Config {
        port: 3456,
        ..Config::debug_default()
    };

    let allowed_origins = AllowedOrigins::all();

    let cors = CorsOptions {
        allowed_origins,
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: false,
        ..Default::default()
    }
    .to_cors()
    .unwrap();

    rocket::build()
        .configure(config)
        .attach(cors)
        .manage(channel::<FeederEvent>(10).0)
        .mount("/api/v1", api::routes())
        .mount("/", FileServer::from(relative!("static")))
}
