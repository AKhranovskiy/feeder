mod classifier;
mod score;

use rocket::{fairing, Rocket};

pub use self::classifier::Classifier;

pub use score::AveragePerSecondScore;

pub struct IgniteClassifier;

#[rocket::async_trait]
impl fairing::Fairing for IgniteClassifier {
    fn info(&self) -> fairing::Info {
        use fairing::Kind;

        fairing::Info {
            name: "Audio Content classifier",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<rocket::Build>) -> fairing::Result {
        match Classifier::new() {
            Ok(cl) => Ok(rocket.manage(cl)),
            Err(ref error) => {
                log::error!("{error:#}");
                Err(rocket)
            }
        }
    }
}

const INPUT_CHUNK_DURATION_SEC: usize = 4;
