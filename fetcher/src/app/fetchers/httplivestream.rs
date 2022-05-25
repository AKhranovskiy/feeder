use std::time::Duration;

use anyhow::Result;
use async_stream::try_stream;
use reqwest::Url;
use tokio_stream::Stream;

pub struct HttpLiveStreamingFetcher {
    source: Url,
    client: reqwest::Client,
}

impl HttpLiveStreamingFetcher {
    pub fn new(source: Url) -> Self {
        Self {
            source,
            client: reqwest::Client::new(),
        }
    }

    /// Receive the next message published on a subscribed channel, waiting if
    /// necessary.
    ///
    /// `None` indicates the subscription has been terminated.
    pub async fn next_segment(&mut self) -> Result<Option<u32>> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        // match self.client.connection.read_frame().await? {
        //     Some(mframe) => {
        //         debug!(?mframe);

        //         match mframe {
        //             Frame::Array(ref frame) => match frame.as_slice() {
        //                 [message, channel, content] if *message == "message" => Ok(Some(Message {
        //                     channel: channel.to_string(),
        //                     content: Bytes::from(content.to_string()),
        //                 })),
        //                 _ => Err(mframe.to_error()),
        //             },
        //             frame => Err(frame.to_error()),
        //         }
        //     }
        //     None => Ok(None),
        // }
        Ok(Some(20))
    }

    /// Convert the subscriber into a `Stream` yielding new messages published
    /// on subscribed channels.
    ///
    /// `Subscriber` does not implement stream itself as doing so with safe code
    /// is non trivial. The usage of async/await would require a manual Stream
    /// implementation to use `unsafe` code. Instead, a conversion function is
    /// provided and the returned stream is implemented with the help of the
    /// `async-stream` crate.
    pub fn fetch(mut self) -> impl Stream<Item = Result<u32>> {
        log::trace!(target: "HttpLiveStreamingFetcher", "Fetching source={}", &self.source);
        // Uses the `try_stream` macro from the `async-stream` crate. Generators
        // are not stable in Rust. The crate uses a macro to simulate generators
        // on top of async/await. There are limitations, so read the
        // documentation there.
        try_stream! {
            while let Some(segment) = self.next_segment().await? {
                yield segment
            }
        }
    }
}
