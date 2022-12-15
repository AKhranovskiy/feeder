use tch::{nn, Tensor};

/// https://www.projectpro.io/article/music-genre-classification-project-python-code/566
pub fn cnn_projectpro(vs: &nn::Path) -> nn::SequentialT {
    nn::seq_t()
        //
        .add(nn::conv2d(vs.sub("first"), 1, 32, 3, Default::default()))
        .add_fn(|x| x.relu())
        .add_fn(|x| x.max_pool2d_default(2))
        //
        .add(nn::conv2d(
            vs.sub("second"),
            32,
            128,
            3,
            Default::default(),
        ))
        .add_fn(|x| x.relu())
        .add_fn(|x| x.max_pool2d_default(2))
        .add_fn_t(|x, train| x.dropout(0.3, train))
        //
        .add(nn::conv2d(
            vs.sub("third"),
            128,
            128,
            3,
            Default::default(),
        ))
        .add_fn(|x| x.relu())
        .add_fn(|x| x.max_pool2d_default(2))
        .add_fn_t(|x, train| x.dropout(0.3, train))
        //
        // .add_fn(print_size)
        // .add_fn(|x| x.adaptive_avg_pool2d(&[2,2]).flatten(1,-1))
        .add_fn(|x| x.mean_dim(Some(&[2,3][..]), false, tch::Kind::Float))
        //
        // .add_fn(print_size)
        .add(nn::linear(vs.sub("dense 1"), 128, 512, Default::default()))
        .add_fn(|x| x.relu())
        //
        .add(nn::linear(vs.sub("dense 2"), 512, 2, Default::default()))
        // .add_fn(print_size)
        .add_fn(|x| x.softmax(-1, tch::Kind::Float))
        .add_fn(print_tensor)
}

#[allow(dead_code)]
#[inline(always)]
fn print_tensor(t: &Tensor) -> Tensor {
    t.print();
    t.shallow_clone()
}

#[allow(dead_code)]
#[inline(always)]
fn print_size(t: &Tensor) -> Tensor {
    println!("{:?}", t.size());
    t.shallow_clone()
}
