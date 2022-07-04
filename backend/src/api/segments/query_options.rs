#[derive(Debug, FromForm)]
pub struct QueryOptions<'r> {
    pub skip: Option<u64>,
    pub limit: Option<i64>,
    pub kind: Option<&'r str>,
}
