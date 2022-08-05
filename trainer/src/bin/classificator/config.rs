use std::path::PathBuf;

#[derive(Debug)]
pub struct ClassificationConfig {
    pub input_weights_filename: PathBuf,
    pub audio_file: PathBuf,
    pub dry_run: bool,
}
