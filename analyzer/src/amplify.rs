use classifier::PredictedLabels;

pub trait Apmlify {
    fn amplified(self, coefficients: &[f32]) -> Self;
}

impl Apmlify for PredictedLabels {
    fn amplified(self, coefficients: &[f32]) -> Self {
        assert!((1.0 - self.iter().sum::<f32>()).abs() < 1e-6);
        assert_eq!(self.shape(), [1, coefficients.len()]);
        assert!(coefficients.iter().all(|f| *f >= 0.0));

        let coeff_sum = coefficients.iter().sum::<f32>();
        assert!(coeff_sum > 0.0);

        let normalized_coeffs = coefficients
            .iter()
            .map(|c| *c / coeff_sum)
            .collect::<Vec<_>>();

        let amplified_values =
            self * Self::from_shape_vec((1, normalized_coeffs.len()), normalized_coeffs).unwrap();

        let amplified_sum = amplified_values.iter().sum::<f32>();
        assert!(amplified_sum > 0.0);

        amplified_values / amplified_sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_amplified() {
        let sut = array![[0.1, 0.7, 0.2]].amplified(&[0.33, 1.00, 0.10]);
        assert_eq!(sut, array![[0.043_824_706, 0.929_614_9, 0.026_560_428]]);
    }
}
