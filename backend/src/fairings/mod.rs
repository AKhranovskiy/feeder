mod analyser;
mod check_db;
mod check_emysound;
mod classification;
mod fetcher;
mod playback;

use hls_fetcher::Segment;
use rocket::fairing::AdHoc;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Live Stream Analyser", |rocket| async move {
        let (rs_tx, rs_rx) = flume::bounded(10);
        rocket
            .attach(check_db::CheckDb)
            .attach(check_emysound::CheckEmySound)
            .attach(classification::IgniteClassifier)
            .manage(RawSegmentTx(rs_tx))
            .manage(RawSegmentRx(rs_rx))
            .attach(analyser::Analyser)
            .attach(fetcher::Fetcher)
            .attach(playback::PlaybackPruner)
    })
}

#[derive(Debug, Clone)]
pub(crate) struct RawSegmentPacket {
    stream_id: String,
    // TODO - Use model::Segment.
    segment: Segment,
}

impl RawSegmentPacket {
    pub(crate) fn new(stream_id: String, segment: Segment) -> Self {
        Self { stream_id, segment }
    }
}

pub(crate) struct RawSegmentTx(flume::Sender<RawSegmentPacket>);
pub(crate) struct RawSegmentRx(flume::Receiver<RawSegmentPacket>);
