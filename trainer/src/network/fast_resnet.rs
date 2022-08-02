use tch::nn;

pub fn fast_resnet(vs: &nn::Path) -> nn::SequentialT {
    nn::seq_t()
        .add(conv_bn(&vs.sub("pre"), 1, 64))
        .add(layer(&vs.sub("layer1"), 64, 128))
        .add(conv_bn(&vs.sub("inter"), 128, 256))
        .add_fn(|x| x.max_pool2d_default(2))
        .add(layer(&vs.sub("layer2"), 256, 512))
        .add_fn(|x| x.max_pool2d_default(4).flat_view())
        .add(nn::linear(&vs.sub("linear"), 2560, 3, Default::default()))
        .add_fn(|x| x * 0.125)
}

fn conv_bn(vs: &nn::Path, c_in: i64, c_out: i64) -> nn::SequentialT {
    let conv2d_cfg = nn::ConvConfig {
        padding: 1,
        bias: false,
        ..Default::default()
    };
    nn::seq_t()
        .add(nn::conv2d(vs, c_in, c_out, 3, conv2d_cfg))
        .add(nn::batch_norm2d(vs, c_out, Default::default()))
        .add_fn(|x| x.relu())
}

fn layer<'a>(vs: &nn::Path, c_in: i64, c_out: i64) -> nn::FuncT<'a> {
    let pre = conv_bn(&vs.sub("pre"), c_in, c_out);
    let block1 = conv_bn(&vs.sub("b1"), c_out, c_out);
    let block2 = conv_bn(&vs.sub("b2"), c_out, c_out);
    nn::func_t(move |xs, train| {
        let pre = xs.apply_t(&pre, train).max_pool2d_default(2);
        let ys = pre.apply_t(&block1, train).apply_t(&block2, train);
        pre + ys
    })
}
