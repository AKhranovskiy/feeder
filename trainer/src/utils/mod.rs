pub mod data;
mod indices;
mod stat;

pub use indices::random_bucket_indices;
pub use stat::RunningAverage;
pub use stat::Stats;

#[inline(always)]
pub fn ensure_finite(t: &tch::Tensor, msg: &str) -> tch::Tensor {
    let is_fin: bool = t.isfinite().totype(tch::Kind::Bool).all().into();
    assert!(is_fin, "{msg}");
    t.shallow_clone()
}
