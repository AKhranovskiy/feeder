use classifier::PredictedLabels;
use ndarray::{Array1, Axis};

pub trait Apmlify {
    fn amplified(self, coefficients: &[f32]) -> Self;
}

impl Apmlify for PredictedLabels {
    fn amplified(mut self, coefficients: &[f32]) -> Self {
        assert!(coefficients.iter().all(|f| *f >= 0.0));

        let coeff_sum = coefficients.iter().sum::<f32>();
        assert!(coeff_sum > 0.0);

        let normalized_coeffs = coefficients
            .iter()
            .map(|c| *c / coeff_sum)
            .collect::<Vec<_>>();

        for mut t in self.axis_iter_mut(Axis(0)) {
            assert_eq!(t.shape(), [coefficients.len(),]);
            assert!((1.0 - t.iter().sum::<f32>()).abs() < 1e-6);

            let amplified_values = t.to_owned() * Array1::from_vec(normalized_coeffs.clone());
            let amplified_sum = amplified_values.iter().sum::<f32>();
            assert!(amplified_sum > 0.0);

            t.assign(&(amplified_values / amplified_sum));
        }
        self
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
