use rocket::get;
use rocket_dyn_templates::{context, Template};

#[get("/streams")]
pub async fn streams() -> Template {
    Template::render("streams", context! {})
}
