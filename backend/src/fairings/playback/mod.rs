use std::time::Duration;

use rocket::{fairing, Rocket};
use rocket_db_pools::Database;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tokio::select;
use tokio::time::interval;

use crate::internal::storage::Storage;
use crate::storage::StorageScheme;

pub struct PlaybackPruner;

const PRUNE_INTERVAL: Duration = Duration::from_secs(30);
const PRUNE_AGE: Duration = Duration::from_secs(60 * 10);

#[rocket::async_trait]
impl fairing::Fairing for PlaybackPruner {
    fn info(&self) -> fairing::Info {
        use fairing::Kind;

        fairing::Info {
            name: "Playback pruner",
            kind: Kind::Liftoff | Kind::Singleton,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        log::info!("Playback pruner starting");

        let playbacks = (*Storage::fetch(rocket).expect("Storage must be initialized."))
            .clone()
            .playbacks();

        let mut shutdown = rocket.shutdown();
        let mut interval = interval(PRUNE_INTERVAL);

        tokio::spawn(async move {
            loop {
                select! {
                    _ = interval.tick() => {
                        let till = std::time::SystemTime::now() - PRUNE_AGE;
                        log::info!("Prunning playbacks till {}", OffsetDateTime::from(till).format(&Iso8601::DEFAULT).unwrap());
                        if let Err(ref error) = playbacks.prune(till).await {
                            log::error!("{error}");
                        }
                    },
                    _ = &mut shutdown => {
                        log::info!("Analyser shutting down");
                        break;
                    },
                    else => break,
                };
            }
        });
    }
}
