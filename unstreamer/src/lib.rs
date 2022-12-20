mod hls;

use std::io::Read;

use anyhow::bail;
use url::Url;

static MIME_AUDIO: &str = "audio/";

#[non_exhaustive]
pub struct Unstreamer(Box<dyn Read>);

impl Unstreamer {
    pub fn open(source: Url) -> anyhow::Result<Unstreamer> {
        let resp = ureq::get(source.as_ref()).call()?;
        if resp.content_type() == hls::MIME_HLS {
            Ok(Self(Box::new(hls::HLSUnstreamer::open(source)?)))
        } else if resp.content_type().starts_with(MIME_AUDIO) {
            Ok(Self(resp.into_reader()))
        } else {
            bail!("Unsupported content type: {}", resp.content_type());
        }
    }
}

impl Read for Unstreamer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
