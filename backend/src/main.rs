#![feature(decl_macro)]

use rocket::form::Form;
use rocket::form::Strict;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::Config;

#[macro_use]
extern crate rocket;

#[derive(FromForm)]
struct SegmentsUpload<'r> {
    json: &'r str,
    content: TempFile<'r>,
}

#[post("/api/v1/segments/upload", data = "<upload>")]
async fn segments_upload(upload: Form<Strict<SegmentsUpload<'_>>>) -> Status {
    println!("json: {:?}", upload.json);
    println!("content: {}", upload.content.len());
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    let config = Config {
        port: 3456,
        ..Config::debug_default()
    };

    rocket::build()
        .configure(config)
        .mount("/", routes![segments_upload])
}
