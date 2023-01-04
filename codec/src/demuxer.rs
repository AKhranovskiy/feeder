use std::io::Read;

use ac_ffmpeg::format::demuxer::Demuxer as AcDemuxer;
use ac_ffmpeg::format::demuxer::DemuxerWithStreamInfo as AcDemuxerWithStreamInfo;
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::format::stream::Stream;
use ac_ffmpeg::packet::Packet;

#[non_exhaustive]
pub struct Demuxer<T>(AcDemuxerWithStreamInfo<T>);

impl<R: Read> Demuxer<R> {
    pub fn try_from(input: R) -> anyhow::Result<Self> {
        let io = IO::from_read_stream(input);

        let demuxer = AcDemuxer::builder()
            .build(io)?
            .find_stream_info(None)
            .map_err(|(_, err)| err)?;

        Ok(Self(demuxer))
    }
}

impl<T> Demuxer<T> {
    pub(crate) fn stream(&self) -> &Stream {
        &self.0.streams()[0]
    }
}

impl<T> Iterator for Demuxer<T> {
    type Item = anyhow::Result<Packet>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map_err(Into::into).transpose()
    }
}
