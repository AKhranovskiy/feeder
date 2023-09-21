use std::sync::Arc;

use crate::{ads_management::AdsProvider, args::Args, terminate::Terminator};

#[derive(Clone)]
pub struct AppState {
    pub terminator: Terminator,
    pub ads_provider: Arc<AdsProvider>,
    pub args: Args,
}
