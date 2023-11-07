use clap::Args;

use training::database::Database;

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct RunArgs {
    /// By name (latest timestamp).
    name: Option<String>,
    /// By hash
    hash: Option<String>,
}

impl RunArgs {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        let Some(name) = &self.name else {
            anyhow::bail!("No model name or hash provided");
        };

        #[allow(clippy::cast_possible_wrap)]
        let hash = u64::from_str_radix(name, 16).ok().map(|h| h as i64);

        let Some(model) = db.find_model(Some(name), hash).await? else {
            anyhow::bail!("Model '{}' not found", name);
        };

        println!(
            "{:<15} {:>16x} {} {:>10}",
            model.name,
            model.hash,
            model.timestamp.to_rfc3339(),
            model.size
        );

        let run_id = db.start_model_run(model.hash).await?;

        let file_hashes = db.select_file_indices_for_run(run_id).await?;
        println!("found {} files", file_hashes.len());

        println!(
            "{:<15} {:>16x} {} {:>10}",
            model.name,
            model.hash,
            model.timestamp.to_rfc3339(),
            model.size
        );

        todo!()
    }
}
