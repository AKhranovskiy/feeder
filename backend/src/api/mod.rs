mod segments;

use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![segments::upload::upload]
}
