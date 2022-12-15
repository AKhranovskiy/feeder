use tch::nn;

use crate::utils::ensure_finite;

/// https://docs.microsoft.com/en-us/learn/modules/intro-audio-classification-pytorch/4-speech-model
pub fn cnn_ms(vs: &nn::Path) -> nn::SequentialT {
    nn::seq_t()
        // First conv
        .add(nn::conv2d(
            &vs.sub("first conv"),
            1,
            32,
            5,
            Default::default(),
        ))
        .add_fn(|x| x.max_pool2d_default(2))
        .add_fn(|x| x.relu())
        .add_fn(|x| ensure_finite(x, "first conv"))
        // Second conv
        .add(nn::conv2d(
            &vs.sub("second conv"),
            32,
            64,
            5,
            Default::default(),
        ))
        .add_fn_t(|x, train| x.dropout(0.5, train))
        .add_fn(|x| x.max_pool2d_default(2))
        .add_fn(|x| x.relu())
        .add_fn(|x| ensure_finite(x, "second conv"))
        // Flatten
        .add_fn(|x| x.flatten(1, -1))
        .add_fn(|x| ensure_finite(x, "flatten"))
        // First linear
        .add(nn::linear(
            &vs.sub("first linear"),
            18048,
            50,
            Default::default(),
        ))
        .add_fn(|x| x.relu())
        .add_fn(|x| ensure_finite(x, "first linear"))
        // Dropout
        .add_fn_t(|x, train| x.dropout(0.5, train))
        .add_fn(|x| x.relu())
        .add_fn(|x| ensure_finite(x, "dropout"))
        // Second linear
        .add(nn::linear(
            &vs.sub("second linear"),
            50,
            2,
            Default::default(),
        ))
        .add_fn(|x| x.relu())
        .add_fn(|x| ensure_finite(x, "second linear"))
        // .add_fn(|x| {x.print(); x.shallow_clone()})
        // Softmax
        // .add_fn(|x| x.log_softmax(1, tch::Kind::Float))
        // .add_fn(|x| ensure_finite(x, "final"))
}
