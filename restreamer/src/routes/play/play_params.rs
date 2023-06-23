use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PlayParams {
    pub source: String,
    pub action: Option<PlayAction>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlayAction {
    Passthrough,
    Silence,
    Replace,
}
