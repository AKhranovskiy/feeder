use futures::future::try_join;
use futures::{StreamExt, TryFutureExt};
use model::ContentKind;
use rocket::http::ContentType;
use rocket::{fairing, Rocket};
use rocket_db_pools::Database;
use tokio::select;

use crate::fairings::classification::{AveragePerSecondScore, Classifier};
use crate::internal::analyse_tags;
use crate::internal::codec::prepare_for_browser;
use crate::internal::storage::Storage;
use crate::storage::{playback::Playback, StorageScheme};

use super::RawSegmentRx;

pub struct Analyser;

#[rocket::async_trait]
impl fairing::Fairing for Analyser {
    fn info(&self) -> fairing::Info {
        use fairing::Kind;

        fairing::Info {
            name: "Analyser",
            kind: Kind::Liftoff | Kind::Singleton,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        log::info!("Analyser starting");

        let client = (*Storage::fetch(rocket).expect("Storage must be initialized.")).clone();
        let playbacks = client.playbacks();

        let mut segments = rocket
            .state::<RawSegmentRx>()
            .expect("RawSegment channel must be set up.")
            .0
            .clone()
            .into_stream();

        let mut shutdown = rocket.shutdown();

        let classifier: Classifier = rocket
            .state::<Classifier>()
            .expect("Classifier is ignited")
            .clone();

        tokio::spawn(async move {
            loop {
                let packet = select! {
                    Some(packet) = segments.next() => packet,
                    _ = &mut shutdown => {
                        log::info!("Analyser shutting down");
                        break;
                    },
                    else => break,
                };

                let result = try_join(
                    analyse_tags(&packet.segment.content, &packet.segment.comment),
                    classifier.classify(&packet.segment.content, AveragePerSecondScore),
                )
                .and_then(|((tags, _), classification)| {
                    let classification = classification.iter().map(|p| p.max()).collect();

                    let content_type = ContentType::parse_flexible(&packet.segment.content_type)
                        .unwrap_or(ContentType::Binary);

                    let (content_type, content) =
                        prepare_for_browser(&content_type, &packet.segment.content)
                            .unwrap_or((content_type, packet.segment.content.clone()));

                    let p = Playback::new(
                        packet.stream_id,
                        content_type.to_string(),
                        content,
                        tags.track_title_or_empty(),
                        tags.track_artist_or_empty(),
                        packet.segment.duration,
                        classification,
                    );
                    log::info!(
                        "New segment from {}: {} - {}, {:.02}s, {}",
                        p.stream_id,
                        p.artist,
                        p.title,
                        p.duration.as_secs_f32(),
                        p.classification.iter().map(short_kind).collect::<String>()
                    );
                    playbacks.add(p)
                })
                .await;

                if let Err(ref error) = result {
                    log::error!("{error:#}");
                }
            }
        });
    }
}

fn short_kind((kind, _): &(ContentKind, f32)) -> char {
    kind.to_string()
        .chars()
        .next()
        .expect("Content kind string is not empty")
}
