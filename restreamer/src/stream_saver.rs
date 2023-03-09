use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use codec::{AudioFrame, CodecParams, Encoder};
use time::{format_description, macros::offset, OffsetDateTime};

const BASE_PATH: &str = "./recordings";

#[derive(Debug, Clone, Copy)]
pub enum Destination {
    Original,
    Processed,
}

impl Destination {
    pub fn into_path(self) -> PathBuf {
        let format =
            format_description::parse("[year][month][day]-[hour][minute][second]").unwrap();
        let now = OffsetDateTime::now_utc()
            .to_offset(offset!(+7))
            .format(&format)
            .unwrap();

        let dest = match self {
            Destination::Original => "original",
            Destination::Processed => "processed",
        };

        Path::new(BASE_PATH).join(format!("{now}-{dest}.ogg"))
    }
}

pub struct StreamSaver {
    original: Encoder<BufWriter<File>>,
    processed: Encoder<BufWriter<File>>,
}

impl StreamSaver {
    pub fn new(codec_params: CodecParams) -> anyhow::Result<Self> {
        eprintln!(
            "Creating stream saver\n{}\n{}",
            Destination::Original.into_path().display(),
            Destination::Processed.into_path().display()
        );

        let original = {
            let writer = BufWriter::new(File::create(Destination::Original.into_path())?);
            Encoder::opus(codec_params, writer)?
        };
        let processed = {
            let writer = BufWriter::new(File::create(Destination::Processed.into_path())?);
            Encoder::opus(codec_params, writer)?
        };

        Ok(Self {
            original,
            processed,
        })
    }

    pub fn push(&mut self, destination: Destination, frame: AudioFrame) {
        let pts = frame.pts();
        match destination {
            Destination::Original => {
                let pts = frame.pts();
                if let Err(error) = self.original.push(frame) {
                    eprintln!("Failed to save original frame {pts:?}: {error:#?}");
                }
            }
            Destination::Processed => {
                if let Err(error) = self.processed.push(frame) {
                    eprintln!("Failed to save procesed frame {pts:?}: {error:#?}");
                }
            }
        }
    }

    pub fn flush(&mut self) {
        self.original.flush().unwrap();
        self.processed.flush().unwrap();
    }
}
