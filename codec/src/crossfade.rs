use std::cmp::Ordering;

#[derive(Debug, Default, Clone, Copy)]
pub struct CrossFadePair(pub f64, pub f64);

impl PartialEq for CrossFadePair {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
            && self.1.total_cmp(&other.1) == Ordering::Equal
    }
}

impl Eq for CrossFadePair {}

impl From<(f64, f64)> for CrossFadePair {
    fn from(pair: (f64, f64)) -> Self {
        Self::new(pair.0, pair.1)
    }
}

impl CrossFadePair {
    pub fn new(fade_out: f64, fade_in: f64) -> Self {
        Self(fade_out, fade_in)
    }

    pub fn apply(&self, left: f64, right: f64) -> f64 {
        self.0 * left + self.1 * right
    }

    pub fn fade_out(&self) -> f64 {
        self.0
    }
    pub fn fade_in(&self) -> f64 {
        self.1
    }
}

pub trait CrossFade {
    fn generate(size: usize) -> Vec<CrossFadePair>;
}

pub struct EqualPowerCrossFade;

impl CrossFade for EqualPowerCrossFade {
    fn generate(size: usize) -> Vec<CrossFadePair> {
        // https://signalsmith-audio.co.uk/writing/2021/cheap-energy-crossfade/

        let step = 1.0f64 / (size - 1) as f64;

        (0..size)
            .map(|n| {
                let x = step * (n as f64);
                let x2 = 1_f64 - x;
                let a = x * x2;
                let b = a + 1.4186_f64 * a.powi(2);
                let fin = (b + x).powi(2);
                let fout = (b + x2).powi(2);
                (fout, fin).into()
            })
            .collect()
    }
}

pub struct LinearCrossFade;

impl CrossFade for LinearCrossFade {
    fn generate(size: usize) -> Vec<CrossFadePair> {
        let step = 1.0f64 / (size - 1) as f64;
        (0..size)
            .map(|n| n as f64)
            .map(|n| (1_f64 - step * n, step * n).into())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_power_cross_fade() {
        assert_eq!(
            EqualPowerCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (1.0029835420672355, 0.04059848606723561).into(),
                (0.9926458906771458, 0.15706649867714562).into(),
                (0.9458734593312678, 0.3278252513312678).into(),
                (0.8495518311530496, 0.5208672871530496).into(),
                (0.7033547889062499, 0.7033547889062499).into(), // middle
                (0.5208672871530495, 0.8495518311530498).into(),
                (0.3278252513312675, 0.9458734593312678).into(),
                (0.15706649867714553, 0.9926458906771456).into(),
                (0.04059848606723558, 1.0029835420672355).into(),
                (0.0, 1.0).into(),
            ]
        );
    }

    #[test]
    fn test_linear_cross_fade() {
        assert_eq!(
            LinearCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (0.9, 0.1).into(),
                (0.8, 0.2).into(),
                (0.7, 0.30000000000000004).into(),
                (0.6, 0.4).into(),
                (0.5, 0.5).into(), // middle
                (0.3999999999999999, 0.6000000000000001).into(),
                (0.29999999999999993, 0.7000000000000001).into(),
                (0.19999999999999996, 0.8).into(),
                (0.09999999999999998, 0.9).into(),
                (0.0, 1.0).into(),
            ],
        );
    }
}
