use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use codec::{AudioFrame, Decoder, FrameDuration};

use super::listener::PlayEventListener;

#[derive(Clone)]
pub struct AdEntry {
    pub(super) name: String,
    pub(super) frames: Vec<AudioFrame>,
    pub(super) duration: Duration,
    pub(super) event_listener: PlayEventListener,
    pub(super) next_player_id: Arc<AtomicUsize>,
}

impl Debug for AdEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdEntry")
            .field("name", &self.name)
            .field("frames", &self.frames.len())
            .field("duration", &self.duration)
            .field(
                "next_player_id",
                &self.next_player_id.load(Ordering::Relaxed),
            )
            .field("events", &self.event_listener.events().len())
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

        let event_listener = PlayEventListener::new(frames.len());

        Ok(Self {
            name,
            frames,
            duration,
            event_listener,
            next_player_id: Arc::new(AtomicUsize::new(1)),
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
            event_listener: PlayEventListener::new(0),
            next_player_id: Arc::new(AtomicUsize::new(1)),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::AdEntry;

    #[test]
    fn test_event_listener() {
        // 626 frames
        let entry = AdEntry::try_from(Path::new("sample.mp3")).unwrap();
        let quarter = 156;
        let mid = 313;
        let third = 469;
        let total = 626;

        let mut iter = entry.into_iter();
        assert!(entry.event_listener.events().is_empty());

        iter.next();
        assert_eq!(1, entry.event_listener.events().len());

        for _ in 2..quarter {
            iter.next();
        }
        assert_eq!(1, entry.event_listener.events().len());

        iter.next();
        assert_eq!(2, entry.event_listener.events().len());

        for _ in quarter + 2..mid {
            iter.next();
        }
        assert_eq!(2, entry.event_listener.events().len());

        iter.next();
        assert_eq!(3, entry.event_listener.events().len());

        for _ in mid + 1..third {
            iter.next();
        }
        assert_eq!(3, entry.event_listener.events().len());

        iter.next();
        assert_eq!(4, entry.event_listener.events().len());

        for _ in third..total {
            iter.next();
        }
        assert_eq!(4, entry.event_listener.events().len());

        iter.next();
        assert_eq!(5, entry.event_listener.events().len());

        for _ in 0..total {
            iter.next();
        }
        assert_eq!(5, entry.event_listener.events().len());
    }
}
