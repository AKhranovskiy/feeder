use trainer::networks::Network;
use trainer::utils::Stats;

use crate::config::ClassificationConfig;

pub fn classify(
    _network: &Network,
    _config: &ClassificationConfig,
    _images: &tch::Tensor,
) -> anyhow::Result<(tch::Tensor, Stats)> {
    unimplemented!("Under refactoring")
    // let (vs, loaded) = network.create_varstore(&config.input_weights_filename);
    // loaded?;
    //
    // let net = network.create_network(&vs.root());
    //
    // let labels = tch::Tensor::zeros(&[images.size()[0], 3], tch::kind::FLOAT_CPU);
    //
    // let mut timings = Stats::new();
    //
    // for (idx, image) in tqdm!(
    //     images.split(1, 0).iter().enumerate(),
    //     desc = "Classifying",
    //     animation = "fillup",
    //     unit = "image",
    //     disable = false
    // ) {
    //     let timer = Instant::now();
    //
    //     labels.i(idx as i64).copy_(
    //         &net.forward_t(image, /*train=*/ false)
    //             .softmax(-1, tch::Kind::Float)
    //             .squeeze(),
    //     );
    //
    //     timings = timings.push(timer.elapsed().as_secs_f64());
    // }
    //
    // Ok((labels, timings))
}
