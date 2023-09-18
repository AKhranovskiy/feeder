use std::{
    collections::{hash_map::Entry, HashMap},
    io::Cursor,
    sync::Arc,
};

use anyhow::anyhow;
use codec::{AudioFrame, CodecParams, Decoder, FrameDuration, Resampler};
use futures::future::try_join_all;
use std::time::Duration;
use tokio::sync::RwLock;

type Track = Vec<AudioFrame>;

struct CacheItem {
    params: CodecParams,
    track: Track,
    duration: Duration,
}

type ResampledCache = HashMap<(usize, CodecParams), Arc<Track>>;

pub struct AdCache {
    tracks: Vec<CacheItem>,
    resampled: Arc<RwLock<ResampledCache>>,
}

impl AdCache {
    pub async fn build(tracks: &[Vec<u8>]) -> anyhow::Result<Self> {
        let tracks = try_join_all(tracks.iter().cloned().map(|track| {
            tokio::task::spawn_blocking(move || {
                let decoder = Decoder::try_from(Cursor::new(&track))?;
                let mut params = decoder.codec_params();

                let track = decoder.collect::<Result<Vec<_>, _>>()?;

                if let Some(frame) = track.first() {
                    params = params.with_samples_per_frame(frame.samples());
                }

                let duration = track
                    .iter()
                    .fold(Duration::ZERO, |acc, frame| acc + frame.duration());

                anyhow::Ok(CacheItem {
                    params,
                    track,
                    duration,
                })
            })
        }))
        .await?
        // Unwrap inner results.
        .into_iter()
        .collect::<Result<_, _>>()?;

        Ok(Self {
            tracks,
            resampled: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    #[allow(dead_code)]
    pub fn content(&self) -> Vec<(usize, Duration)> {
        self.tracks
            .iter()
            .enumerate()
            .map(|(id, item)| (id, item.duration))
            .collect()
    }

    pub fn get(&self, id: usize, target_params: CodecParams) -> anyhow::Result<Arc<Track>> {
        let item = self
            .tracks
            .get(id)
            .ok_or_else(|| anyhow!("Track not found"))?;

        let key = (id, target_params);

        // Fast check with reader lock.
        if let Ok(reader) = self.resampled.try_read() {
            if reader.contains_key(&key) {
                return Ok(reader.get(&key).unwrap().clone());
            }
        }

        // Full check with writer lock.
        let mut writer = self.resampled.blocking_write();

        #[allow(clippy::significant_drop_in_scrutinee)]
        match writer.entry(key) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
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

                Ok(entry.insert(Arc::new(resampled_track)).clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build() {
        let cache = AdCache::build(&[include_bytes!("../sample.aac").to_vec()])
            .await
            .expect("Ad cache");

        assert_eq!(1, cache.tracks.len());
    }

    #[tokio::test]
    async fn test_get() {
        const TARGET_PARAMS: CodecParams =
            CodecParams::new(44100, codec::SampleFormat::FltPlanar, 2).with_samples_per_frame(512);

        let cache = AdCache::build(&[include_bytes!("../sample.aac").to_vec()])
            .await
            .expect("Ad cache");

        let track_a = cache.get(0, TARGET_PARAMS).expect("Track A");
        let track_b = cache.get(0, TARGET_PARAMS).expect("Track B");
        let track_c = cache
            .get(0, TARGET_PARAMS.with_samples_per_frame(128))
            .expect("Track C");

        assert!(Arc::ptr_eq(&track_a, &track_b));
        assert!(!Arc::ptr_eq(&track_a, &track_c));
    }
}
