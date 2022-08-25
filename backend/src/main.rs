#![feature(decl_macro)]
#![feature(fs_try_exists)]

#[macro_use]
extern crate rocket;

use frontend::extend_tera;
use rocket::fs::{relative, FileServer};
use rocket::tokio::sync::broadcast::channel;
// use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use rocket_db_pools::Database;

mod api;
mod frontend;
pub mod internal;

use api::FeederEvent;
use internal::storage::Storage;
use rocket_dyn_templates::Template;

#[launch]
fn rocket() -> _ {
    // TODO compilation error in async-trait macro
    // let allowed_origins = AllowedOrigins::all();

    // let cors = CorsOptions {
    //     allowed_origins,
    //     allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
    //     allow_credentials: false,
    //     ..Default::default()
    // }
    // .to_cors()
    // .unwrap();

    rocket::build()
        // .attach(cors)
        .attach(Storage::init())
        // .attach(Template::fairing())
        .attach(Template::custom(|engines| extend_tera(&mut engines.tera)))
        .manage(channel::<FeederEvent>(10).0)
        .mount("/", frontend::routes())
        .mount("/api/v1", api::routes())
        .mount("/static", FileServer::from(relative!("static")))
}
