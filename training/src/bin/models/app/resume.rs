use std::{path::Path, sync::Arc};

use chrono::Utc;
use clap::Args;
use classifier::AdbandaModel;
use kdam::BarExt;
use tap::TapFallible;
use tokio::{sync::Mutex, task::JoinSet};
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
            "adbanda-model-{}-{}-{:x}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            model.name,
            model.hash,
        );

        tar::Archive::new(content.as_slice()).unpack(dir.join(&name))?;

        let pb = progress_bar(file_hashes.len())?;

        let model = Arc::new(classifier::AdbandaModel::load(dir, &name)?);

        let (sender, receiver) = flume::unbounded();

        for hash in file_hashes {
            sender.send_async(hash).await?;
        }

        let mut workers = JoinSet::new();
        for _ in 0..6 {
            let db = db.clone();
            let model = model.clone();
            let pb = pb.clone();
            let receiver = receiver.clone();
            workers.spawn(async move { process_embedding(db, model, pb, receiver, run.id).await });
        }

        while !sender.is_empty() {
            tokio::task::yield_now().await;
        }

        println!("Drop the sender");
        drop(sender);

        println!("Join workers");
        while let Some(item) = workers.join_next().await {
            item??;
        }

        Ok(())
    }
}

fn progress_bar(total: usize) -> anyhow::Result<Arc<Mutex<kdam::Bar>>> {
    kdam::BarBuilder::default()
        .total(total)
        .unit("embeddings")
        .desc("Classified")
        .force_refresh(true)
        .build()
        .map_err(|s| anyhow::anyhow!(s))
        .map(Mutex::new)
        .map(Arc::new)
}

async fn process_embedding(
    db: Database,
    model: Arc<AdbandaModel>,
    pb: Arc<Mutex<kdam::Bar>>,
    receiver: flume::Receiver<i64>,
    run_id: i64,
) -> anyhow::Result<()> {
    while let Ok(hash) = receiver.recv_async().await {
        if let Some(embedding) = db
            .get_embedding(hash)
            .await
            .tap_err(|e| println!("Error getting embedding: {e}"))?
        {
            let predictions = model
                .run(embedding)
                .tap_err(|e| println!("Error running model: {e}"))?;

            let dist = analyzer::dist(&predictions)
                .tap_err(|err| println!("Error computing distance: {err}"))?;

            let confidence = analyzer::confidence(&dist);
            assert!(confidence.shape() == [3]);

            db.insert_file_score(
                run_id,
                hash,
                predictions.as_slice().unwrap(),
                predictions.shape(),
                confidence.as_slice().unwrap().try_into().unwrap(),
            )
            .await
            .tap_err(|e| println!("Error inserting score: {e}"))?;
        }

        pb.lock().await.update(1).tap_err(|e| {
            println!("Error updating progress bar: {e}");
        })?;
    }
    Ok(())
}
