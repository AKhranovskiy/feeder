use std::path::Path;

use clap::{Args, Subcommand};

use training::database::Database;

#[derive(Args)]
pub struct GetArgs {
    #[command(subcommand)]
    target: GetTarget,
}

#[derive(Subcommand)]
enum GetTarget {
    /// Get model
    Model {
        /// Model hash
        hash: String,
    },
    /// Get file.
    File {
        /// File hash.
        hash: String,
    },
}

impl GetArgs {
    pub async fn run(&self, db: &Database) -> anyhow::Result<()> {
        match self.target {
            GetTarget::Model { hash: _ } => todo!(),
            GetTarget::File { ref hash } => {
                #[allow(clippy::cast_possible_wrap)]
                let Some(hash) = u64::from_str_radix(hash, 16).ok().map(|h| h as i64) else {
                    anyhow::bail!("Invalid file hash");
                };

                let Some((name, content)) = db.get_file(hash).await? else {
                    anyhow::bail!("File {hash} not found");
                };

                let name = Path::new(&name).file_name().unwrap();
                let path = Path::new("/run/user/1000/").join(name);
                tokio::fs::write(&path, content).await?;
                println!("File exported to {}", path.display());
            }
        }

        Ok(())
    }
}
