use std::path::Path;

use clap::ArgEnum;
use tch::nn::{self, OptimizerConfig, SequentialT};

use super::cnn_ms::cnn_ms;
use super::cnn_projectpro::cnn_projectpro;
use super::fast_resnet::fast_resnet;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum Network {
    /// Fast ResNet architecture, from rust-tch.
    FastResNet,
    /// CNN architecture from Microsoft.
    CnnMs,
    /// CNN architecture from ProjectPro.
    CnnPp,
}

impl Network {
    pub fn create_network(&self, path: &nn::Path) -> SequentialT {
        match self {
            Network::FastResNet => fast_resnet(path),
            Network::CnnMs => cnn_ms(path),
            Network::CnnPp => cnn_projectpro(path),
        }
    }

    pub fn create_varstore(&self, weight_file: &Path) -> (nn::VarStore, anyhow::Result<()>) {
        let mut varstore = nn::VarStore::new(tch::Device::cuda_if_available());
        let loaded = varstore.load(Path::new(&weight_file));
        (varstore, loaded.map_err(anyhow::Error::msg))
    }

    pub fn learning_rate(&self, epoch: usize) -> f64 {
        if epoch < 50 {
            0.1
        } else if epoch < 100 {
            0.01
        } else {
            0.001
        }
    }

    pub fn create_optimizer(&self, vs: &nn::VarStore) -> anyhow::Result<nn::Optimizer> {
        // TODO - use network-specific optimizer.
        nn::Sgd {
            momentum: 0.9,
            dampening: 0.,
            wd: 5e-4,
            nesterov: true,
        }
        .build(vs, 0.)
        .map_err(|e| e.into())
    }
}
