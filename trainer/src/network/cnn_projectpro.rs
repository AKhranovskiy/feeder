use tch::nn;

/// https://www.projectpro.io/article/music-genre-classification-project-python-code/566
pub fn cnn_projectpro(vs: &nn::Path) -> nn::SequentialT {
    nn::seq_t()
        .add(nn::conv2d(&vs.sub("first"), 1, 32, 3, Default::default()))
        .add_fn(|x| x.relu().max_pool2d_default(2))
        .add(nn::conv2d(
            &vs.sub("second"),
            32,
            128,
            3,
            Default::default(),
        ))
        .add_fn(|x| x.relu().max_pool2d_default(2).dropout(0.3, true))
        .add(nn::conv2d(
            &vs.sub("third"),
            128,
            128,
            3,
            Default::default(),
        ))
        .add_fn(|x| x.relu().max_pool2d_default(2).dropout(0.3, true))
        .add_fn(|x| x.adaptive_avg_pool2d(&[2, 2]).flatten(1, -1))
        .add(nn::linear(&vs.sub("dense 1"), 512, 128, Default::default()))
        .add_fn(|x| x.relu())
        .add(nn::linear(&vs.sub("dense 2"), 128, 3, Default::default()))
        .add_fn(|x| x.softmax(1, tch::Kind::Float))
}
