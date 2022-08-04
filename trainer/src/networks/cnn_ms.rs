use tch::nn;

/// https://docs.microsoft.com/en-us/learn/modules/intro-audio-classification-pytorch/4-speech-model
pub fn cnn_ms(vs: &nn::Path) -> nn::SequentialT {
    nn::seq_t()
        .add(nn::conv2d(&vs.sub("first"), 1, 32, 5, Default::default()))
        .add_fn(|x| x.max_pool2d_default(2).relu())
        .add(nn::conv2d(&vs.sub("second"), 32, 64, 5, Default::default()))
        .add_fn(|x| x.dropout(0.5, true).max_pool2d_default(2).relu())
        .add_fn(|x| x.flatten(1, -1))
        .add(nn::linear(
            &vs.sub("linear 1"),
            14976,
            50,
            Default::default(),
        ))
        .add_fn(|x| x.relu())
        .add_fn(|x| x.dropout(0.5, true).relu())
        .add(nn::linear(&vs.sub("linear 1"), 50, 3, Default::default()))
        .add_fn(|x| x.log_softmax(1, tch::Kind::Float))
}
