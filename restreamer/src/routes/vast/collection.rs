use std::sync::Arc;

use rand::Rng;

const VAST_FILES: &[&str] = &[include_str!("../../../vast/one.xml")];

#[derive(Debug, Clone)]
pub struct VastCollection(Arc<Vec<String>>);

impl VastCollection {
    pub fn new(server: &str) -> Self {
        let items = VAST_FILES
            .iter()
            .map(|item| item.replace("{{SERVER}}", server))
            .collect();
        Self(Arc::new(items))
    }

    pub fn get_random(&self) -> String {
        let id = rand::thread_rng().gen_range(0..self.0.len());
        self.0[id].clone()
    }
}
