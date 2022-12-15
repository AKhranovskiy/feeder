use std::io::Cursor;

use bson::serde_helpers::time_0_3_offsetdatetime_as_bson_datetime;
use kdam::{tqdm, BarExt};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tch::nn::{ModuleT, Optimizer, SequentialT, VarStore};
use tch::IndexOp;

use crate::data;
pub use crate::networks::Network;
use crate::stat::Stats;

#[derive(Debug)]
pub struct Classificator {
    network_info: NetworkInfo,
    vs: VarStore,
    seq: SequentialT,
    optimizer: Optimizer,
    accuracy: f64,
}

impl Classificator {
    pub fn empty(network: Network) -> Self {
        configure_cuda();

        let vs = network.create_varstore();
        let seq = network.create_network(&vs.root());
        let optimizer = network
            .create_optimizer(&vs)
            .expect("Optimizer should always be available");

        Self {
            network_info: NetworkInfo::new(network, 1),
            vs,
            seq,
            optimizer,
            accuracy: 0.0,
        }
    }

    pub fn with_model(model: Model) -> anyhow::Result<Self> {
        let mut classificator = Classificator::empty(model.network_info.architecture);

        anyhow::ensure!(
            classificator.network_info == model.network_info,
            "Network architecture version mismatch: expected: {}, given={}",
            model.network_info.version,
            classificator.network_info.version
        );

        classificator
            .vs
            .load_from_stream(Cursor::new(model.model.as_slice()))?;

        classificator.accuracy = model.accuracy;

        Ok(classificator)
    }

    pub fn model(&self) -> anyhow::Result<Model> {
        let mut model = vec![];
        self.vs.save_to_stream(&mut model)?;

        Ok(Model {
            date: time::OffsetDateTime::now_utc(),
            accuracy: self.accuracy,
            model,
            network_info: self.network_info,
        })
    }

    pub async fn classify(&self, audio_data: Vec<u8>) -> anyhow::Result<()> {
        let images = data::prepare_classification_images(audio_data).await?;
        let labels = tch::Tensor::zeros(
            &[images.size()[0], 2],
            (tch::Kind::Float, tch::Device::cuda_if_available()),
        );

        for (idx, image) in images.split(1, 0).iter().enumerate() {
            labels.i(idx as i64).copy_(
                &self
                    .seq
                    .forward_t(image, /*train=*/ false)
                    .softmax(-1, tch::Kind::Float)
                    .squeeze(),
            );
        }

        labels.print();
        Ok(())
    }

    pub async fn train(&mut self, audio_data: Vec<u8>, _class: u8) -> anyhow::Result<f64> {
        let _data = data::prepare_train_images(audio_data).await?;
        todo!()
    }

    pub async fn batch_train(&mut self, mfccs: Vec<Vec<f32>>) -> anyhow::Result<f64> {
        let dataset = data::prepare_batch_train_dataset(mfccs).await?;

        const TRAIN_BATCH_SIZE: i64 = 10;
        const TEST_BATCH_SIZE: i64 = 2;

        let mut epoch_pb = tqdm!(
            total = 10,
            desc = "Training",
            animation = "fillup",
            unit = "epoch",
            force_refresh = true,
            disable = true
        );

        let mut accuracy = Stats::new();

        for epoch in 0..10 {
            self.optimizer
                .set_lr(self.network_info.architecture.learning_rate(epoch));

            for (bimages, blabels) in tqdm!(
                dataset
                    .train_iter(TRAIN_BATCH_SIZE)
                    .to_device(tch::Device::cuda_if_available())
                    .shuffle(),
                desc = "Batches",
                unit = "batch",
                force_refresh = true,
                position = 1,
                disable = true
            ) {
                let loss = self
                    .seq
                    .forward_t(&bimages, true)
                    .cross_entropy_for_logits(&blabels);
                self.optimizer.backward_step(&loss);

                let loss: f64 = loss.into();
                println!("Loss = {loss:1.06}");
            }

            let test_accuracy = self.seq.batch_accuracy_for_logits(
                &dataset.test_images,
                &dataset.test_labels,
                tch::Device::cuda_if_available(),
                TEST_BATCH_SIZE,
            );

            accuracy = accuracy.push(test_accuracy);
            epoch_pb.write(format!("EPOCH {epoch:>3}: {:>3.2}%", test_accuracy * 100.0));
            epoch_pb.update(1);
        }

        self.accuracy = accuracy.last().unwrap_or_default();

        let result: Vec<(u8, u8)> = dataset
            .test_iter(1)
            .to_device(tch::Device::cuda_if_available())
            .shuffle()
            .map(|(image, label)| {
                let prediction = self
                    .seq
                    .forward_t(&image, /*train=*/ false)
                    .softmax(-1, tch::Kind::Float)
                    .squeeze();

                let (_, classes) = prediction.max_dim(-1, false);
                let label = u8::from(&label);
                let class = u8::from(&classes);
                (label, class)
            })
            .collect();

        let correct = result.iter().filter(|(a, b)| a == b).count() as f32;
        let total = dataset.test_images.size()[0] as f32;

        println!(
            "Final validation: {:>3.2}%: {:?}",
            correct * 100.0 / total,
            result.into_iter().map(|(_, a)| a).collect::<Vec<_>>()
        );

        Ok(self.accuracy)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub struct NetworkInfo {
    architecture: Network,
    version: u32,
}

impl NetworkInfo {
    pub fn new(architecture: Network, version: u32) -> Self {
        Self {
            architecture,
            version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde_as]
pub struct Model {
    #[serde(with = "time_0_3_offsetdatetime_as_bson_datetime")]
    date: time::OffsetDateTime,
    accuracy: f64,
    #[serde_as(as = "Bytes")]
    model: Vec<u8>,
    network_info: NetworkInfo,
}

fn configure_cuda() {
    if tch::Cuda::is_available() {
        println!("Found CUDA device");
        tch::Cuda::cudnn_set_benchmark(true);
    }
}
