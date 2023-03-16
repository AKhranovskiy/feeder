use crate::terminate::Terminator;

use super::collection::VastCollection;

#[derive(Clone)]
pub struct VastState {
    pub collection: VastCollection,
    #[allow(dead_code)]
    pub terminator: Terminator,
}
