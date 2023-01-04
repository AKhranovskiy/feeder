use serde::{Deserialize, Serialize};
use tch::nn::{self, OptimizerConfig, SequentialT};

use super::cnn_ms::cnn_ms;
use super::cnn_projectpro::cnn_projectpro;
use super::fast_resnet::fast_resnet;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Network {
    /// Fast ResNet architecture, from rust-tch.
    FastResNet,
    /// CNN architecture from Microsoft.
    CnnMs,
    /// CNN architecture from ProjectPro.
    CnnPp,
}

impl Network {
    pub(crate) fn create_network(&self, path: &nn::Path) -> SequentialT {
        match self {
            Network::FastResNet => fast_resnet(path),
            Network::CnnMs => cnn_ms(path),
            Network::CnnPp => cnn_projectpro(path),
        }
    }

    pub(crate) fn create_varstore(&self) -> nn::VarStore {
        nn::VarStore::new(tch::Device::cuda_if_available())
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

    pub(crate) fn create_optimizer(&self, vs: &nn::VarStore) -> anyhow::Result<nn::Optimizer> {
        // TODO - use network-specific optimizer.
        nn::Adam::default().build(vs, 0.1).map_err(|e| e.into())
    }
}