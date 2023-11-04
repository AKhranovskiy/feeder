use std::path::Path;

use chrono::Utc;
use clap::Args;
use training::Database;

#[derive(Args)]
pub struct ResumeArgs {
    run_id: i64,
}

impl ResumeArgs {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        let Some(run) = db.find_model_run(self.run_id).await? else {
            anyhow::bail!("Model run not found");
        };

        println!("Run: {run}");

        let file_hashes = db.select_file_indices_for_run(run.id).await?;
        println!("found {} files", file_hashes.len());

        if file_hashes.is_empty() {
            if run.finished.is_none() {
                db.complete_model_run(run.id).await?;
            }
            println!("Model run finished");
            return Ok(());
        }

        let Some(model) = db.find_model(None, Some(run.model_hash)).await? else {
            anyhow::bail!("Model '{}':{} not found", run.model_name, run.model_hash);
        };

        println!("Model: {model}");

        let Some(content) = db.get_model_content(model.hash).await? else {
            anyhow::bail!("Model content not found");
        };

        let dir = Path::new("/run/user/1000/");

        let name = format!(
            "adbanda-model-{}-{:x}-{}",
            model.name,
            model.hash,
            Utc::now().timestamp()
        );

        tar::Archive::new(content.as_slice()).unpack(dir.join(&name))?;

        let _ = classifier::AdbandaModel::load(dir, &name)?;

        Ok(())
    }
}
