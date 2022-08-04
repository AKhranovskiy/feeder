use std::path::PathBuf;

pub struct PredictionConfig {
    pub input_weights_file: PathBuf,
    pub data_directory: PathBuf,
    pub samples: Option<usize>,
    pub dry_run: bool,
}
