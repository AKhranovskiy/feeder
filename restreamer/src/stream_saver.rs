use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use chrono::Local;
use codec::{AudioFrame, CodecParams, Encoder};

const BASE_PATH: &str = "./recordings";

#[derive(Debug, Clone, Copy)]
pub enum Destination {
    Original,
    Processed,
}

impl Destination {
    pub fn into_path(self) -> PathBuf {
        let now = Local::now().format("%Y%m%d-%H%M%S");

        let dest = match self {
            Destination::Original => "original",
            Destination::Processed => "processed",
        };

        Path::new(BASE_PATH).join(format!("{now}-{dest}.ogg"))
    }
}

pub struct StreamSaver {
    original: Option<Encoder<BufWriter<File>>>,
    processed: Option<Encoder<BufWriter<File>>>,
}

impl StreamSaver {
    pub fn new(enabled: bool, codec_params: CodecParams) -> anyhow::Result<Self> {
        if enabled {
            log::info!(
                "Creating stream saver\n{}\n{}",
                Destination::Original.into_path().display(),
                Destination::Processed.into_path().display()
            );

            let original = Some({
                let writer = BufWriter::new(File::create(Destination::Original.into_path())?);
                Encoder::opus(codec_params, writer)?
            });
            let processed = Some({
                let writer = BufWriter::new(File::create(Destination::Processed.into_path())?);
                Encoder::opus(codec_params, writer)?
            });

            Ok(Self {
                original,
                processed,
            })
        } else {
            log::info!("Recordings are not enabled");
            Ok(Self {
                original: None,
                processed: None,
            })
        }
    }

    pub fn push(&mut self, destination: Destination, frame: AudioFrame) {
        let pts = frame.pts();
        match destination {
            Destination::Original if self.original.is_some() => {
                let pts = frame.pts();
                if let Err(error) = self.original.as_mut().unwrap().push(frame) {
                    log::error!("Failed to save original frame {pts:?}: {error:#?}");
                }
            }
            Destination::Processed if self.processed.is_some() => {
                if let Err(error) = self.processed.as_mut().unwrap().push(frame) {
                    log::error!("Failed to save procesed frame {pts:?}: {error:#?}");
                }
            }
            _ => {}
        }
    }

    pub fn flush(&mut self) {
        self.original.as_mut().map(Encoder::flush);
        self.processed.as_mut().map(Encoder::flush);
    }
}
