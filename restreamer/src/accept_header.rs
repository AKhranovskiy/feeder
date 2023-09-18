use core::fmt;
use std::str::FromStr;

use axum::{
    headers::{self, Header},
    http::{header::ACCEPT, HeaderName, HeaderValue},
};
use mime::Mime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accept(Vec<Mime>);

impl Header for Accept {
    fn name() -> &'static HeaderName {
        &ACCEPT
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(
        values: &mut I,
    ) -> Result<Self, headers::Error> {
        values
            .next()
            .and_then(|v| {
                v.to_str()
                    .ok()?
                    .split(',')
                    .map(Mime::from_str)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()
            })
            .map(Accept)
            .ok_or_else(headers::Error::invalid)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let mimes = self
            .0
            .iter()
            .map(Mime::as_ref)
            .map(HeaderValue::from_str)
            .collect::<Result<Vec<_>, _>>()
            .expect("Mime is always a valid HeaderValue");
        values.extend(mimes);
    }
}

impl fmt::Display for Accept {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(
            &self
                .0
                .iter()
                .map(Mime::as_ref)
                .collect::<Vec<_>>()
                .join(","),
            f,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multiple_mimes() {
        let input =
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8";
        let accept =
            Accept::decode(&mut std::iter::once(&HeaderValue::from_str(input).unwrap())).unwrap();

        let expexted = Accept(vec![
            Mime::from_str("text/html").unwrap(),
            Mime::from_str("application/xhtml+xml").unwrap(),
            Mime::from_str("application/xml;q=0.9").unwrap(),
            Mime::from_str("image/avif").unwrap(),
            Mime::from_str("image/webp").unwrap(),
            Mime::from_str("*/*;q=0.8").unwrap(),
        ]);

        assert_eq!(accept, expexted);
    }
}
