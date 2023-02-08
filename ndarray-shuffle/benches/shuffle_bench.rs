use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ndarray::Axis;
use ndarray_shuffle::NdArrayShuffleInplaceExt;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut sut =
        ndarray::Array3::from_shape_vec((1500, 150, 39), (0..1500 * 150 * 39).collect::<Vec<_>>())
            .unwrap();

    c.bench_function("shuffle 1500x150x39, Axis 0", |b| {
        b.iter(|| sut.shuffle_inplace(black_box(Axis(0))));
    });

    c.bench_function("shuffle 1500x150x39, Axis 1", |b| {
        b.iter(|| sut.shuffle_inplace(black_box(Axis(1))));
    });

    c.bench_function("shuffle 1500x150x39, Axis 2", |b| {
        b.iter(|| sut.shuffle_inplace(black_box(Axis(2))));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
