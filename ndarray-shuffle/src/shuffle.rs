use ndarray::{ArrayBase, Axis, Dimension, OwnedRepr};
pub use rand::Rng;

use crate::shuffle_inplace::NdArrayShuffleInplaceExt;
use crate::{default_rng, NdArrayShuffleError};

pub trait NdArrayShuffleExt {
    type Output;
    fn shuffle(&self, axis: Axis) -> Result<Self::Output, NdArrayShuffleError>;
    fn shuffle_with<R>(&self, axis: Axis, rng: &mut R) -> Result<Self::Output, NdArrayShuffleError>
    where
        R: Rng + ?Sized;
}

impl<A, D> NdArrayShuffleExt for ArrayBase<OwnedRepr<A>, D>
where
    A: Clone + std::fmt::Debug + Send,
    D: Dimension,
{
    type Output = Self;

    fn shuffle(&self, axis: Axis) -> Result<Self::Output, NdArrayShuffleError> {
        self.shuffle_with(axis, &mut default_rng())
    }

    fn shuffle_with<R>(&self, axis: Axis, rng: &mut R) -> Result<Self::Output, NdArrayShuffleError>
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
            return Ok(self.clone());
        }

        let mut output = self.clone();
        output.shuffle_inplace_with(axis, rng).map(|_| output)
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn static_rng() -> SmallRng {
        SmallRng::seed_from_u64(0xFEEB)
    }

    use ndarray::Axis;

    use crate::shuffle::NdArrayShuffleError;

    use super::NdArrayShuffleExt;

    #[test]
    fn test_empty_array() {
        let sut = ndarray::Array3::<u64>::default((0, 0, 0));
        assert_eq!(sut, sut.shuffle_with(Axis(1), &mut static_rng()).unwrap());
    }

    #[test]
    fn test_array_invalid_axis() {
        let sut = ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(
            NdArrayShuffleError::InvalidAxis(3),
            sut.shuffle_with(Axis(3), &mut static_rng()).unwrap_err()
        );
    }

    #[test]
    fn test_array_non_standard_layout() {
        let sut = ndarray::Array3::from_shape_vec((3, 3, 3), (1..=3 * 3 * 3).collect::<Vec<_>>())
            .unwrap()
            .reversed_axes();

        assert_eq!(
            NdArrayShuffleError::NonStandardLayout,
            sut.shuffle_with(Axis(0), &mut static_rng()).unwrap_err()
        );
    }
}
