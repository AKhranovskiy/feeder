use std::{
    collections::{hash_map::Entry, HashMap},
    io::Cursor,
    sync::Arc,
};

use anyhow::bail;
use codec::{AudioFrame, CodecParams, Decoder, FrameDuration, Resampler};
use std::time::Duration;
use tokio::sync::RwLock;

type Track = Vec<AudioFrame>;

#[derive(Clone)]
struct TrackCacheItem {
    params: CodecParams,
    track: Track,
    #[allow(dead_code)]
    duration: Duration,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct AdId(uuid::Uuid);

impl AdId {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl AsRef<uuid::Uuid> for AdId {
    fn as_ref(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl From<uuid::Uuid> for AdId {
    fn from(id: uuid::Uuid) -> Self {
        Self(id)
    }
}

type TrackCache = HashMap<AdId, TrackCacheItem>;
type ResampledCache = HashMap<(AdId, CodecParams), Arc<Track>>;

type ArcLock<T> = Arc<RwLock<T>>;

pub struct AdCache {
    tracks: ArcLock<TrackCache>,
    resampled: ArcLock<ResampledCache>,
}

impl AdCache {
    pub fn new() -> Self {
        Self {
            tracks: Arc::new(RwLock::new(TrackCache::new())),
            resampled: Arc::new(RwLock::new(ResampledCache::new())),
        }
    }

    #[cfg(test)]
    pub async fn ids(&self) -> Vec<AdId> {
        self.tracks.read().await.keys().copied().collect()
    }

    pub async fn insert(&self, id: AdId, track: &[u8]) -> anyhow::Result<()> {
        #[allow(clippy::significant_drop_in_scrutinee)]
        match self.tracks.write().await.entry(id) {
            Entry::Occupied(_) => {
                bail!("Track already exists");
            }
            Entry::Vacant(entry) => {
                let decoder = Decoder::try_from(Cursor::new(track))?;
                let mut params = decoder.codec_params();

                let track = decoder.collect::<Result<Vec<_>, _>>()?;

                if let Some(frame) = track.first() {
                    params = params.with_samples_per_frame(frame.samples());
                }

                let duration = track
                    .iter()
                    .fold(Duration::ZERO, |acc, frame| acc + frame.duration());

                entry.insert(TrackCacheItem {
                    params,
                    track,
                    duration,
                });

                Ok(())
            }
        }
    }

    #[cfg(test)]
    pub async fn testing(tracks: &[Vec<u8>]) -> anyhow::Result<Self> {
        let this = Self::new();
        futures::future::try_join_all(tracks.iter().map(|track| this.insert(AdId::new(), track)))
            .await?;
        Ok(this)
    }

    pub async fn get(
        &self,
        id: AdId,
        target_params: CodecParams,
    ) -> anyhow::Result<Option<Arc<Track>>> {
        log::debug!(
            "Get advertisement, id={}, params={target_params:?}",
            id.as_ref().to_string()
        );
        let key = (id, target_params);

        // Fast check with reader lock.
        if let Ok(reader) = self.resampled.try_read() {
            if reader.contains_key(&key) {
                log::debug!("Found in cache");
                return Ok(Some(reader.get(&key).unwrap().clone()));
            }
        }

        let Some(item) = self.tracks.read().await.get(&id).cloned() else {
            return Ok(None);
        };

        let mut writer = self.resampled.write().await;

        #[allow(clippy::significant_drop_in_scrutinee)]
        match writer.entry(key) {
            Entry::Occupied(entry) => {
                log::debug!("Found in cache");
                Ok(Some(entry.get().clone()))
            }
            Entry::Vacant(entry) => {
                let source_track = item.track.clone();
                let source_params = item.params;

                let resampled_track = {
                    let mut resampler = Resampler::new(source_params, target_params);
                    let mut frames = vec![];
                    for source_frame in source_track {
                        for frame in resampler.push(source_frame)? {
                            frames.push(frame?);
                        }
                    }
                    anyhow::Ok(frames)
                }?;

                log::debug!("Resampled");
                Ok(Some(entry.insert(Arc::new(resampled_track)).clone()))
            }
        }
    }
}

#[cfg(test)]
impl AdCache {
    pub fn build_testing(id: AdId, track: Vec<AudioFrame>) -> Self {
        Self {
            tracks: Arc::new(RwLock::new(
                std::iter::once((
                    id,
                    TrackCacheItem {
                        params: super::CODEC_PARAMS,
                        track: track.clone(),
                        duration: Duration::from_secs(track.len() as u64),
                    },
                ))
                .collect(),
            )),
            resampled: Arc::new(RwLock::new(
                std::iter::once(((id, super::CODEC_PARAMS), Arc::new(track))).collect(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build() {
        let cache = AdCache::testing(&[include_bytes!("../../sample.aac").to_vec()])
            .await
            .expect("Ad cache");

        assert_eq!(1, cache.ids().await.len());
    }

    #[tokio::test]
    async fn test_get() {
        const TARGET_PARAMS: CodecParams =
            CodecParams::new(44100, codec::SampleFormat::FltPlanar, 2).with_samples_per_frame(512);

        let cache = AdCache::testing(&[include_bytes!("../../sample.aac").to_vec()])
            .await
            .expect("Ad cache");

        let id = cache.ids().await[0];
        let track_a = cache
            .get(id, TARGET_PARAMS)
            .await
            .expect("Track A")
            .expect("Track A");
        let track_b = cache
            .get(id, TARGET_PARAMS)
            .await
            .expect("Track B")
            .expect("Track B");
        let track_c = cache
            .get(id, TARGET_PARAMS.with_samples_per_frame(128))
            .await
            .expect("Track C")
            .expect("Track C");

        assert!(Arc::ptr_eq(&track_a, &track_b));
        assert!(!Arc::ptr_eq(&track_a, &track_c));
    }
}
