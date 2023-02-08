use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::Read;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, ensure};
use flume::{Receiver, TryRecvError};
use hls_m3u8::tags::VariantStream;
use hls_m3u8::{MasterPlaylist, MediaPlaylist, MediaSegment};
use url::Url;

pub(crate) static MIME_HLS: &str = "application/vnd.apple.mpegurl";

pub(crate) struct HLSUnstreamer {
    data_rx: Receiver<Box<(dyn Read + Send + Sync)>>,
    error_rx: Receiver<anyhow::Error>,
    readers: VecDeque<Box<(dyn Read + Send + Sync)>>,
}

impl HLSUnstreamer {
    pub(crate) fn open(source: Url) -> anyhow::Result<Box<dyn Read + Send>> {
        let resp = ureq::get(source.as_ref()).call()?;

        ensure!(
            resp.content_type() == MIME_HLS,
            "Invalid content type: {}",
            resp.content_type()
        );

        // TODO check for content size to avoid overflows.
        // The content can be either master playlist or media playlist.

        let content = resp.into_string()?;

        let source = MasterPlaylist::try_from(content.as_ref())
            .map_err(Into::into)
            .and_then(get_best_media_playlist_url)
            .or(anyhow::Ok(source))?;

        let (data_tx, data_rx) = flume::unbounded();
        let (error_tx, error_rx) = flume::bounded::<anyhow::Error>(1);

        let mut last_fetched = 0;

        std::thread::spawn(move || loop {
            if data_tx.is_disconnected() {
                eprintln!("Data is disconected, exit");
                break;
            }

            let playlist = match fetch_media_playlist(&source) {
                Ok(playlist) => playlist,
                Err(error) => {
                    let _ = error_tx.send(error);
                    break;
                }
            };

            match extract_segments(&playlist) {
                Ok(segments) => {
                    for segment in segments {
                        if segment.sequence_number <= last_fetched {
                            continue;
                        }

                        last_fetched = segment.sequence_number;

                        if data_tx.is_disconnected() {
                            eprintln!("Data is disconected, exit");
                            break;
                        }

                        if let Err(error) = ureq::get(segment.source.as_ref()).call().map(|resp| {
                            eprintln!(
                                "Fetched #{}: {}",
                                segment.sequence_number,
                                segment.title.unwrap_or_default()
                            );
                            data_tx.send(resp.into_reader())
                        }) {
                            let _ = error_tx.send(error.into());
                            break;
                        }
                    }
                }
                Err(error) => {
                    let _ = error_tx.send(error);
                    break;
                }
            }

            std::thread::sleep(playlist.duration() / 2);
        });

        Ok(Box::new(Self {
            data_rx,
            error_rx,
            readers: VecDeque::new(),
        }))
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SegmentInfo<'t> {
    sequence_number: usize,
    duration: Duration,
    source: Url,
    title: Option<Cow<'t, str>>,
}

impl<'s> TryFrom<&MediaSegment<'s>> for SegmentInfo<'s> {
    type Error = anyhow::Error;

    fn try_from(segment: &MediaSegment<'s>) -> Result<Self, Self::Error> {
        Ok(Self {
            sequence_number: segment.number(),
            duration: segment.duration.duration(),
            source: Url::from_str(segment.uri().as_ref())?,
            title: segment.duration.title().clone(),
        })
    }
}

fn extract_segments<'p>(playlist: &MediaPlaylist<'p>) -> anyhow::Result<Vec<SegmentInfo<'p>>> {
    playlist
        .segments
        .iter()
        .map(|(_, segment)| segment.try_into())
        .collect()
}

#[allow(clippy::needless_pass_by_value)]
fn get_best_media_playlist_url(master: MasterPlaylist) -> anyhow::Result<Url> {
    master
        .variant_streams
        .iter()
        .filter_map(|vs| match vs {
            VariantStream::ExtXIFrame { .. } => None,
            VariantStream::ExtXStreamInf {
                uri, stream_data, ..
            } => Some((uri, stream_data.bandwidth())),
        })
        .max_by_key(|(_, bandwidth)| *bandwidth)
        .map(|(uri, _)| Url::try_from(uri.as_ref()))
        .transpose()?
        .map(Url::from)
        .ok_or_else(|| anyhow!("Master playlist does not contain any media streams:\n{master:#?}"))
}

fn fetch_media_playlist(source: &Url) -> anyhow::Result<MediaPlaylist> {
    let resp = ureq::get(source.as_ref()).call()?;
    ensure!(
        resp.content_type() == MIME_HLS,
        "Invalid content type: {}",
        resp.content_type()
    );
    let content = resp.into_string()?;
    let playlist = MediaPlaylist::from_str(content.as_ref())?;
    Ok(playlist.into_owned())
}

impl Read for HLSUnstreamer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut total_read = 0usize;

        while !self.readers.is_empty() {
            if total_read == buf.len() {
                break;
            }

            let reader = self
                .readers
                .front_mut()
                .expect("Readers queue is not empty");
            let read = reader.read(&mut buf[total_read..])?;
            total_read += read;

            if read == 0 {
                self.readers.pop_front();
            } else {
                break;
            }
        }

        match self.error_rx.try_recv() {
            Ok(error) => Err(std::io::Error::new(std::io::ErrorKind::Other, error)),
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                TryRecvError::Disconnected,
            )),
        }?;

        match self.data_rx.try_recv() {
            Ok(reader) => {
                self.readers.push_back(reader);
                Ok(())
            }
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                TryRecvError::Disconnected,
            )),
        }?;

        while !self.readers.is_empty() {
            if total_read == buf.len() {
                break;
            }

            let reader = self
                .readers
                .front_mut()
                .expect("Readers queue is not empty");
            let read = reader.read(&mut buf[total_read..])?;

            total_read += read;

            if read == 0 {
                self.readers.pop_front();
            } else {
                break;
            }
        }

        Ok(total_read)
    }
}
