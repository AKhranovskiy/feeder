use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use anyhow::anyhow;
use codec::{AudioFrame, Decoder, FrameDuration};

#[derive(Clone)]
pub struct AdEntry {
    name: String,
    frames: Vec<AudioFrame>,
    duration: Duration,
}

impl Debug for AdEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdEntry")
            .field("name", &self.name)
            .field("frames", &self.frames.len())
            .field("duration", &self.duration)
            .finish()
    }
}

impl TryFrom<&Path> for AdEntry {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let name = path
            .file_name()
            .map(|s| s.to_os_string().into_string())
            .transpose()
            .map_err(|e| anyhow!("Failed to obtain filename: {e:#?}"))?
            .unwrap_or_default();

        let decoder = Decoder::try_from(BufReader::new(File::open(path)?))?;
        // let params = params.with_samples_per_frame(2048); // for OGG
        // let mut resampler = Resampler::new(decoder.codec_params(), params);
        let mut frames = vec![];

        for frame in decoder {
            // for frame in resampler.push(frame?)? {
            frames.push(frame?);
            // }
        }

        let duration = frames.iter().map(FrameDuration::duration).sum();

        Ok(Self {
            name,
            frames,
            duration,
        })
    }
}

impl AdEntry {
    #[cfg(test)]
    pub fn from_name(name: &str) -> Self {
        Self {
            name: name.into(),
            frames: vec![],
            duration: Duration::ZERO,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn frames(&self) -> &[AudioFrame] {
        self.frames.as_ref()
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}
