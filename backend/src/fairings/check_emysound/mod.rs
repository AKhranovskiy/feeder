use rocket::{fairing, Rocket};

use crate::internal::emysound;

pub struct CheckEmySound;

#[rocket::async_trait]
impl fairing::Fairing for CheckEmySound {
    fn info(&self) -> fairing::Info {
        use fairing::Kind;

        fairing::Info {
            name: "Check EmySound connection",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<rocket::Build>) -> fairing::Result {
        match emysound::check_connection() {
            Ok(_) => Ok(rocket),
            Err(ref error) => {
                log::error!("{error:#}");
                Err(rocket)
            }
        }
    }
}
