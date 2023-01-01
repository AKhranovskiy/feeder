use std::time::Instant;

use ndarray::{Array3, Axis};
use ndarray_shuffle::NdArrayShuffleInplaceExt;

fn main() {
    let mut array = Array3::<f32>::default((512, 512, 512));

    let now = Instant::now();
    array.shuffle_inplace(Axis(2)).unwrap();
    let elapsed = now.elapsed();

    println!("Shuffled {:?} in {}ms", array.shape(), elapsed.as_millis());
}
