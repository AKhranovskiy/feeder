use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

#[allow(clippy::expect_used)]
pub fn split_key_values(s: &str) -> HashMap<&str, &str> {
    lazy_static! {
        static ref RE_KEY_VALUE: Regex = Regex::new(
            r#"(\w+)=("([^=]+?)"|\\"([^=]+?)\\"|\\\\"([^=]+?)\\\\"|\\\\\\"([^=]+?)\\\\\\")"#
        )
        .expect("valid regexp");
    }

    RE_KEY_VALUE
        .captures_iter(s)
        .filter_map(|cap| {
            let key = cap.get(1);
            let value = cap
                .get(3)
                .or_else(|| cap.get(4))
                .or_else(|| cap.get(5))
                .or_else(|| cap.get(6));
            key.zip(value).map(|(k, v)| (k.as_str(), v.as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::analysers::ihr::split::split_key_values;

    #[test]
    fn test_empty() {
        assert!(split_key_values("").is_empty());
    }

    #[test]
    fn test_no_key_value() {
        assert!(split_key_values("key").is_empty());
        assert!(split_key_values("key=").is_empty());
        assert!(split_key_values("=value").is_empty());
        assert!(split_key_values("=").is_empty());
    }

    #[test]
    fn test_single_pair_unquoted() {
        assert!(split_key_values("key=value").is_empty());
    }

    #[test]
    fn test_single_pair() {
        assert_eq!(
            split_key_values(r#"key="value""#),
            [("key", "value")].into()
        );
        assert_eq!(
            split_key_values(r#"key=\"value\""#),
            [("key", "value")].into()
        );
        assert_eq!(
            split_key_values(r#"key=\\"value\\""#),
            [("key", "value")].into()
        );
        assert_eq!(
            split_key_values(r#"key=\\\"value\\\""#),
            [("key", "value")].into()
        );
    }

    #[test]
    fn test_multiple_pairs() {
        assert_eq!(
            split_key_values(
                r#"key="value",key2=\"value2\",key3=\\"value3\\" key4=\\\"value4\\\""#
            ),
            [
                ("key", "value"),
                ("key2", "value2"),
                ("key3", "value3"),
                ("key4", "value4"),
            ]
            .into()
        );
    }
}
