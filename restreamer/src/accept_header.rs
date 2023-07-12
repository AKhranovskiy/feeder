use core::fmt;
use std::iter;

use axum::{
    headers::{self, Header},
    http::{header::ACCEPT, HeaderName, HeaderValue},
};
use mime::Mime;

pub(crate) struct Accept(Mime);

impl Header for Accept {
    fn name() -> &'static HeaderName {
        &ACCEPT
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(
        values: &mut I,
    ) -> Result<Self, headers::Error> {
        values
            .next()
            .and_then(|v| v.to_str().ok()?.parse().ok())
            .map(Accept)
            .ok_or_else(headers::Error::invalid)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = self
            .0
            .as_ref()
            .parse()
            .expect("Mime is always a valid HeaderValue");
        values.extend(iter::once(value));
    }
}

impl fmt::Display for Accept {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
