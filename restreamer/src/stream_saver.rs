use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use codec::{AudioFrame, CodecParams, Encoder};
use time::{format_description, macros::offset, OffsetDateTime};

const BASE_PATH: &str = "./recordings";
pub enum Destination {
    Original,
    Processed,
}

impl Destination {
    pub fn path(&self) -> PathBuf {
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

        Path::new(BASE_PATH)
            .join(&format!("{}-{}.ogg", now, dest))
            .to_owned()
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
            Destination::Original.path().display(),
            Destination::Processed.path().display()
        );

        let original = {
            let writer = BufWriter::new(File::create(Destination::Original.path())?);
            Encoder::opus(codec_params, writer)?
        };
        let processed = {
            let writer = BufWriter::new(File::create(Destination::Processed.path())?);
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
                    eprintln!("Failed to save original frame {:?}: {error:#?}", pts);
                }
            }
            Destination::Processed => {
                if let Err(error) = self.processed.push(frame) {
                    eprintln!("Failed to save procesed frame {:?}: {error:#?}", pts);
                }
            }
        }
    }

    pub fn flush(&mut self) {
        self.original.flush().unwrap();
        self.processed.flush().unwrap();
    }
}
