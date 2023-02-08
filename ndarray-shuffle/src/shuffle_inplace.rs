use ndarray::{ArrayBase, Axis, Dimension, OwnedRepr};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
pub use rand::Rng;
use rand::SeedableRng;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;

use crate::{default_rng, NdArrayShuffleError};

pub trait NdArrayShuffleInplaceExt {
    fn shuffle_inplace(&mut self, axis: Axis) -> Result<(), NdArrayShuffleError>;
    fn shuffle_inplace_with<R>(
        &mut self,
        axis: Axis,
        rng: &mut R,
    ) -> Result<(), NdArrayShuffleError>
    where
        R: Rng + ?Sized;
}

impl<A: Send + std::fmt::Debug, D: Dimension> NdArrayShuffleInplaceExt
    for ArrayBase<OwnedRepr<A>, D>
{
    fn shuffle_inplace(&mut self, axis: Axis) -> Result<(), NdArrayShuffleError> {
        self.shuffle_inplace_with(axis, &mut default_rng())
    }

    fn shuffle_inplace_with<R>(
        &mut self,
        axis: Axis,
        rng: &mut R,
    ) -> Result<(), NdArrayShuffleError>
    where
        R: Rng + ?Sized,
    {
        if !self.is_standard_layout() {
            return Err(NdArrayShuffleError::NonStandardLayout);
        }

        if axis.index() >= self.shape().len() {
            return Err(NdArrayShuffleError::InvalidAxis(axis.index()));
        }

        if self.is_empty() {
            return Ok(());
        }

        let len = self.len();
        let shape = self.shape();

        let block = len / shape[0..axis.index()].iter().product::<usize>();
        let sub_block = block / shape[axis.index()];

        let steps = len / block;
        let seeds = (0..steps)
            .map(|_| {
                let mut seed: <SmallRng as SeedableRng>::Seed = Default::default();
                rng.fill(&mut seed);
                seed
            })
            .collect::<Vec<_>>();

        let slice = self.as_slice_mut().expect("Must be a standard layout");

        slice
            .par_chunks_exact_mut(block)
            .enumerate()
            .for_each(|(idx, chunk)| {
                let mut rng = SmallRng::from_seed(seeds[idx]);
                permute_blocks(chunk, &mut rng, sub_block);
            });
        Ok(())
    }
}

#[inline]
fn permute_blocks<T, R>(slice: &mut [T], rng: &mut R, block: usize)
where
    R: Rng + ?Sized,
{
    assert!(block > 0);
    assert!(!slice.is_empty());
    assert!(slice.len() % block == 0);

    if slice.len() == block {
        slice.shuffle(rng);
    } else {
        let number_of_blocks = slice.len() / block;

        for idx in 0..number_of_blocks - 1 {
            let source_start = idx * block;
            let source_end = source_start + block;
            // Give chance to not permute given block.
            let target_start = rng.gen_range(idx..number_of_blocks) * block;
            if source_start != target_start {
                let (left, right) = slice.split_at_mut(target_start);
                left[source_start..source_end].swap_with_slice(&mut right[0..block]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn static_rng() -> SmallRng {
        SmallRng::seed_from_u64(0xFEEB)
    }

    use ndarray::{array, Axis};

    use crate::NdArrayShuffleError;

    use super::NdArrayShuffleInplaceExt;

    #[test]
    fn test_empty_array() {
        let mut sut = ndarray::Array3::<u64>::default((0, 0, 0));
        assert!(sut.shuffle_inplace_with(Axis(1), &mut static_rng()).is_ok());
        assert_eq!(sut, sut);
    }

    #[test]
    fn test_1d_array_axis_0() {
        let mut sut = array![1, 2, 3, 4, 5, 6];
        assert!(sut.shuffle_inplace_with(Axis(0), &mut static_rng()).is_ok());
        assert_eq!(array![5, 6, 4, 2, 1, 3], sut);
    }

    #[test]
    fn test_3d_array_axis_0() {
        let mut sut =
            ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
                .unwrap();
        let expected = array![
            [[19, 20, 21], [22, 23, 24], [25, 26, 27]],
            [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
            [[10, 11, 12], [13, 14, 15], [16, 17, 18]]
        ];

        assert!(sut.shuffle_inplace_with(Axis(0), &mut static_rng()).is_ok());
        assert_eq!(expected, sut);
    }

    #[test]
    fn test_3d_array_axis_1() {
        let mut sut =
            ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
                .unwrap();
        let expected = array![
            [[7, 8, 9], [1, 2, 3], [4, 5, 6]],
            [[16, 17, 18], [10, 11, 12], [13, 14, 15]],
            [[25, 26, 27], [19, 20, 21], [22, 23, 24]]
        ];

        assert!(sut.shuffle_inplace_with(Axis(1), &mut static_rng()).is_ok());
        assert_eq!(expected, sut);
    }

    #[test]
    fn test_3d_array_axis_2() {
        let mut sut =
            ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
                .unwrap();
        let expected = array![
            [[3, 1, 2], [6, 4, 5], [9, 7, 8]],
            [[11, 10, 12], [15, 13, 14], [17, 16, 18]],
            [[21, 19, 20], [22, 23, 24], [25, 26, 27]]
        ];

        assert!(sut.shuffle_inplace_with(Axis(2), &mut static_rng()).is_ok());
        assert_eq!(expected, sut);
    }

    #[test]
    fn test_3d_array_invalid_axis() {
        let mut sut =
            ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
                .unwrap();
        assert_eq!(
            NdArrayShuffleError::InvalidAxis(3),
            sut.shuffle_inplace_with(Axis(3), &mut static_rng())
                .unwrap_err()
        );
    }

    #[test]
    fn test_3d_array_non_standard_layout() {
        let mut sut =
            ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
                .unwrap()
                .reversed_axes();

        assert_eq!(
            NdArrayShuffleError::NonStandardLayout,
            sut.shuffle_inplace_with(Axis(0), &mut static_rng())
                .unwrap_err()
        );
    }
}
