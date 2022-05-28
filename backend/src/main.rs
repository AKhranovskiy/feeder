#![feature(decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::Config;

mod api;

#[launch]
fn rocket() -> _ {
    let config = Config {
        port: 3456,
        ..Config::debug_default()
    };

    rocket::build()
        .configure(config)
        .mount("/api/v1", api::routes())
}
