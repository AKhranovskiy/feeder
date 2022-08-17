// Rocket's FromForm generates code with a warning. It could be already fixed in the latest version of Rocket.
#![allow(clippy::unnecessary_lazy_evaluations)]

#[derive(Debug, FromForm)]
pub struct QueryOptions<'r> {
    pub skip: Option<u64>,
    pub limit: Option<i64>,
    pub kind: Option<&'r str>,
}
