use std::path::PathBuf;

use clap::Args;

use training::database::Database;

#[derive(Args)]
pub struct AddArgs {
    /// Path to model export dir
    path: PathBuf,
}

impl AddArgs {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        let model_file: PathBuf = self.path.join("saved_model.pb");
        if !model_file.exists() {
            anyhow::bail!("{} not found", model_file.display());
        }

        let model_name = self
            .path
            .file_name()
            .and_then(|f| f.to_str())
            .map(ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("Invalid model path '{}'", self.path.display()))?;

        println!("model_name: {model_name}");

        println!("Archiving model...");

        let mut ar = tar::Builder::new(Vec::new());
        ar.append_dir_all(".", &self.path)?;
        let content = ar.into_inner()?;

        println!("Archive size: {}", content.len());

        println!("Inserting into database...");

        db.insert_into_models(&model_name, &content).await?;

        Ok(())
    }
}
