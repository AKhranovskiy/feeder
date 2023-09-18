use std::{cell::Cell, sync::Arc};

use codec::{AudioFrame, CodecParams};

use crate::ad_cache::{AdCache, AdId};

pub struct AdProvider {
    ad_cache: Arc<AdCache>,
    codec_params: CodecParams,
    plan: Vec<AdId>,
    next_item: Cell<usize>,
}

impl AdProvider {
    pub fn new(ad_cache: Arc<AdCache>, codec_params: CodecParams) -> Self {
        let plan = ad_cache
            .content()
            .iter()
            .map(|(id, _)| id)
            .copied()
            .collect();
        Self {
            ad_cache,
            codec_params,
            plan,
            next_item: Cell::new(0),
        }
    }

    pub fn next(&self) -> anyhow::Result<Arc<Vec<AudioFrame>>> {
        let next_item = (self.next_item.get() + 1) % self.plan.len();
        self.next_item.set(next_item);
        assert!(next_item < self.plan.len());

        let next_id = self.plan[next_item];
        self.ad_cache.get(next_id, self.codec_params)
    }
}

#[cfg(test)]
impl AdProvider {
    pub fn new_testing(track: Vec<AudioFrame>) -> Self {
        Self {
            ad_cache: Arc::new(AdCache::build_testing(track)),
            codec_params: AdCache::CODEC_PARAMS,
            plan: vec![AdId::default()],
            next_item: Cell::new(0),
        }
    }
}
