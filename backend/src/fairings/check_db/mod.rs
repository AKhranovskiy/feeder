use rocket::{fairing, Rocket};
use rocket_db_pools::Database;

use crate::internal::storage::Storage;

pub struct CheckDb;

#[rocket::async_trait]
impl fairing::Fairing for CheckDb {
    fn info(&self) -> fairing::Info {
        use fairing::Kind;

        fairing::Info {
            name: "Check DB connection",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<rocket::Build>) -> fairing::Result {
        match Storage::fetch(&rocket) {
            Some(s) => match s.list_database_names(None, None).await {
                Ok(_) => Ok(rocket),
                Err(ref error) => {
                    log::error!("{error:#}");
                    Err(rocket)
                }
            },
            None => Err(rocket),
        }
    }
}
