use std::path::PathBuf;

#[derive(Debug)]
pub struct TrainingConfig {
    // pub input_weights_filename: PathBuf,
    pub output_weights_filename: PathBuf,
    pub input: PathBuf,
    pub test_fraction: f64,
    pub epochs: usize,
    pub dry_run: bool,
}
