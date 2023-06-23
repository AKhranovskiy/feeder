use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Local;
use codec::{AudioFrame, CodecParams, Encoder};

use crate::terminate::Terminator;

#[derive(Debug, Clone, Copy)]
pub enum Destination {
    Original,
    Processed,
}

pub struct StreamSaver {
    inner: Option<Inner>,
}

struct Inner {
    original: flume::Sender<AudioFrame>,
    processed: flume::Sender<AudioFrame>,
    terminator: Terminator,
}

const BASE_PATH: &str = "./recordings";

fn paths() -> (PathBuf, PathBuf) {
    let now = Local::now().format("%Y%m%d-%H%M%S");
    (
        Path::new(BASE_PATH).join(format!("{now}.original.ogg")),
        Path::new(BASE_PATH).join(format!("{now}.processed.ogg")),
    )
}

impl StreamSaver {
    pub fn new(enabled: bool, codec_params: CodecParams) -> anyhow::Result<Self> {
        let inner = if enabled {
            let (original_path, processed_path) = paths();

            log::info!(
                "Creating stream saver\n{}\n{}",
                original_path.display(),
                processed_path.display()
            );

            let terminator = Terminator::new();
            Some(Inner {
                original: start_worker(codec_params, original_path, terminator.clone())?,
                processed: start_worker(codec_params, processed_path, terminator.clone())?,
                terminator,
            })
        } else {
            log::info!("Recordings are not enabled");
            None
        };

        Ok(Self { inner })
    }

    pub fn push(&mut self, destination: Destination, frame: AudioFrame) {
        if let Some(inner) = &mut self.inner {
            let pts = frame.pts();

            match destination {
                Destination::Original => {
                    if let Err(error) = inner.original.send(frame) {
                        log::error!("Failed to save original frame {pts:?}: {error:#?}");
                    }
                }
                Destination::Processed => {
                    if let Err(error) = inner.processed.send(frame) {
                        log::error!("Failed to save procesed frame {pts:?}: {error:#?}");
                    }
                }
            }
        }
    }

    pub fn terminate(&self) {
        if let Some(inner) = &self.inner {
            inner.terminator.terminate();
        }
    }
}

impl Drop for StreamSaver {
    fn drop(&mut self) {
        if let Some(inner) = &self.inner {
            inner.terminator.terminate();
        }
    }
}

const TIMEOUT: Duration = Duration::from_millis(200);

fn start_worker(
    codec_params: CodecParams,
    destination: PathBuf,
    terminator: Terminator,
) -> anyhow::Result<flume::Sender<AudioFrame>> {
    let (sender, queue) = flume::unbounded();

    let mut output = {
        let writer = BufWriter::new(File::create(destination.clone())?);
        Encoder::opus(codec_params, writer)?
    };

    std::thread::spawn(move || {
        while !terminator.is_terminated() {
            while let Ok(frame) = queue.recv_timeout(TIMEOUT) {
                if let Err(error) = output.push(frame) {
                    log::error!(
                        "Failed to save frame for {}: {error:#?}",
                        destination.display()
                    );
                }
            }
        }

        output.flush()?;
        log::info!("Terminating stream saver for {}", destination.display());
        anyhow::Ok(())
    });

    Ok(sender)
}
