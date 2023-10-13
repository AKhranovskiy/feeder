use std::cmp::Ordering;
use std::ops::Mul;

use ac_ffmpeg::codec::audio::AudioFrame;
use bytemuck::{cast_slice, cast_slice_mut};

#[derive(Debug, Default, Clone, Copy)]
pub struct CrossFadePair(f64, f64);

impl Mul<(f32, f32)> for CrossFadePair {
    type Output = f32;

    fn mul(self, rhs: (f32, f32)) -> Self::Output {
        self.0.mul_add(f64::from(rhs.0), self.1 * f64::from(rhs.1)) as Self::Output
    }
}

impl PartialEq for CrossFadePair {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
            && self.1.total_cmp(&other.1) == Ordering::Equal
    }
}

impl Mul<(&AudioFrame, &AudioFrame)> for &CrossFadePair {
    type Output = AudioFrame;

    fn mul(self, (left, right): (&AudioFrame, &AudioFrame)) -> Self::Output {
        assert_eq!(
            left.samples(),
            right.samples(),
            "Frames must have equal number of samples",
        );

        let samples_per_frame = left.samples();

        let left_planes = left.planes();
        let right_planes = right.planes();

        assert_eq!(
            left_planes.len(),
            right_planes.len(),
            "Frames must have equal number of planes",
        );

        let mut frame = left.clone().into_mut();
        let mut planes = frame.planes_mut();

        for i in 0..left_planes.len() {
            let left_data = cast_slice::<_, f32>(left_planes[i].data());
            let right_data = cast_slice::<_, f32>(right_planes[i].data());
            let data = cast_slice_mut::<_, f32>(planes[i].data_mut());

            for x in 0..samples_per_frame {
                data[x] = self.apply(left_data[x], right_data[x]);
            }
        }

        frame.freeze()
    }
}

impl Eq for CrossFadePair {}

impl From<(f64, f64)> for CrossFadePair {
    fn from(pair: (f64, f64)) -> Self {
        Self::new(pair.0.max(0.0), pair.1.max(0.0))
    }
}

impl CrossFadePair {
    pub const BEGIN: Self = Self(1.0, 0.0);
    pub const END: Self = Self(0.0, 1.0);

    #[must_use]
    pub const fn new(fade_out: f64, fade_in: f64) -> Self {
        Self(fade_out, fade_in)
    }

    #[must_use]
    pub fn apply(&self, left: f32, right: f32) -> f32 {
        self.0.mul_add(f64::from(left), self.1 * f64::from(right)) as f32
    }

    #[must_use]
    pub const fn fade_out(&self) -> f64 {
        self.0
    }
    #[must_use]
    pub const fn fade_in(&self) -> f64 {
        self.1
    }
}

pub trait ToFadeInOut {
    #[must_use]
    fn to_fade_in(self) -> Self;

    #[must_use]
    fn to_fade_out(self) -> Self;
}

impl ToFadeInOut for CrossFadePair {
    fn to_fade_in(self) -> Self {
        Self::new(0.0, self.fade_in())
    }

    fn to_fade_out(self) -> Self {
        Self::new(self.fade_out(), 0.0)
    }
}

impl ToFadeInOut for Vec<CrossFadePair> {
    fn to_fade_in(self) -> Self {
        self.into_iter().map(ToFadeInOut::to_fade_in).collect()
    }

    fn to_fade_out(self) -> Self {
        self.into_iter().map(ToFadeInOut::to_fade_out).collect()
    }
}

pub trait CrossFade {
    #[must_use]
    fn step(size: usize) -> f64 {
        if size > 0 {
            1.0f64 / (size - 1) as f64
        } else {
            1.0
        }
    }

    fn generate(size: usize) -> Vec<CrossFadePair> {
        let step = Self::step(size);

        (0..size)
            .map(|n| n as f64 * step)
            .map(Self::calculate)
            .collect()
    }

    fn calculate(x: f64) -> CrossFadePair;
}

pub struct EqualPowerCrossFade;

impl CrossFade for EqualPowerCrossFade {
    fn calculate(x: f64) -> CrossFadePair {
        // https://signalsmith-audio.co.uk/writing/2021/cheap-energy-crossfade/
        let x2 = 1_f64 - x;
        let a = x * x2;
        let b = 1.4186_f64.mul_add(a.powi(2), a);
        let fin = (b + x).powi(2);
        let fout = (b + x2).powi(2);

        (fout, fin).into()
    }
}

pub struct LinearCrossFade;

impl CrossFade for LinearCrossFade {
    fn calculate(x: f64) -> CrossFadePair {
        (1_f64 - x, x).into()
    }
}

pub struct CossinCrossFade;

impl CrossFade for CossinCrossFade {
    fn step(size: usize) -> f64 {
        std::f64::consts::FRAC_PI_2 / (size - 1) as f64
    }

    fn calculate(x: f64) -> CrossFadePair {
        (x.cos().powi(2), x.sin().powi(2)).into()
    }
}

pub struct SemicircleCrossFade;

impl CrossFade for SemicircleCrossFade {
    fn calculate(x: f64) -> CrossFadePair {
        let y1 = if x <= 1_f64 {
            x.mul_add(-x, 1_f64).sqrt()
        } else {
            0_f64
        };

        let y2 = if x >= 1_f64 {
            let a = x - 2_f64;
            a.mul_add(-a, 1_f64).sqrt()
        } else {
            0_f64
        };

        (y1, y2).into()
    }

    fn step(size: usize) -> f64 {
        2.0f64 / (size - 1) as f64
    }
}

pub struct ParabolicCrossFade;

impl CrossFade for ParabolicCrossFade {
    fn calculate(x: f64) -> CrossFadePair {
        (
            3.0f64.mul_add(-x.powi(2), 1.0),
            3.0f64.mul_add(-(x - 1.0).powi(2), 1.0),
        )
            .into()
    }
}

#[cfg(test)]
mod tests {
    use nearly::{
        assert_nearly_eq, EpsTolerance, EpsToleranceType, NearlyEq, NearlyEqEps, NearlyEqTol,
        NearlyEqUlps, UlpsTolerance, UlpsToleranceType,
    };

    use super::*;

    impl UlpsTolerance for CrossFadePair {
        type T = <f64 as UlpsTolerance>::T;
        const DEFAULT: Self::T = <f64 as UlpsTolerance>::DEFAULT;
    }

    impl EpsTolerance for CrossFadePair {
        type T = <f64 as EpsTolerance>::T;
        const DEFAULT: Self::T = <f64 as EpsTolerance>::DEFAULT;
    }

    impl NearlyEqUlps for CrossFadePair {
        fn nearly_eq_ulps(&self, other: &Self, ulps: UlpsToleranceType<Self>) -> bool {
            self.0.nearly_eq_ulps(&other.0, ulps) && self.1.nearly_eq_ulps(&other.1, ulps)
        }
    }

    impl NearlyEqEps for CrossFadePair {
        fn nearly_eq_eps(&self, other: &Self, eps: EpsToleranceType<Self>) -> bool {
            self.0.nearly_eq_eps(&other.0, eps) && self.1.nearly_eq_eps(&other.1, eps)
        }
    }

    impl NearlyEqTol for CrossFadePair {}
    impl NearlyEq for CrossFadePair {}

    #[test]
    fn test_equal_power_cross_fade() {
        assert_nearly_eq!(
            EqualPowerCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (1.002, 0.040).into(),
                (0.992, 0.157).into(),
                (0.945, 0.327).into(),
                (0.849, 0.520).into(),
                (0.703, 0.703).into(), // middle
                (0.520, 0.849).into(),
                (0.327, 0.945).into(),
                (0.157, 0.992).into(),
                (0.040, 1.002).into(),
                (0.0, 1.0).into(),
            ],
            eps = 1e-3
        );
    }

    #[test]
    fn test_linear_cross_fade() {
        assert_nearly_eq!(
            LinearCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (0.9, 0.1).into(),
                (0.8, 0.2).into(),
                (0.7, 0.3).into(),
                (0.6, 0.4).into(),
                (0.5, 0.5).into(), // middle
                (0.4, 0.6).into(),
                (0.3, 0.7).into(),
                (0.2, 0.8).into(),
                (0.1, 0.9).into(),
                (0.0, 1.0).into(),
            ],
            eps = 1e-3
        );
    }

    #[test]
    fn test_cossin_cross_fade() {
        assert_nearly_eq!(
            CossinCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (0.975, 0.024).into(),
                (0.904, 0.095).into(),
                (0.793, 0.206).into(),
                (0.654, 0.345).into(),
                (0.5, 0.5).into(), // middle point
                (0.345, 0.654).into(),
                (0.206, 0.793).into(),
                (0.095, 0.904).into(),
                (0.024, 0.975).into(),
                (0.0, 1.0).into(),
            ],
            eps = 1e-3
        );
    }

    #[test]
    fn test_semicircle_cross_fade() {
        assert_nearly_eq!(
            SemicircleCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (0.979, 0.0).into(),
                (0.916, 0.0).into(),
                (0.8, 0.0).into(),
                (0.6, 0.0).into(),
                (0.0, 0.0).into(),
                (0.0, 0.6).into(),
                (0.0, 0.8).into(),
                (0.0, 0.916).into(),
                (0.0, 0.979).into(),
                (0.0, 1.0).into()
            ],
            eps = 1e-3
        );
    }

    #[test]
    fn test_parabolic_cross_fade() {
        assert_nearly_eq!(
            ParabolicCrossFade::generate(11),
            vec![
                (1.0, 0.0).into(),
                (0.97, 0.0).into(),
                (0.88, 0.0).into(),
                (0.73, 0.0).into(),
                (0.519, 0.0).into(),
                (0.25, 0.25).into(),
                (0.0, 0.520).into(),
                (0.0, 0.730).into(),
                (0.0, 0.880).into(),
                (0.0, 0.97).into(),
                (0.0, 1.0).into()
            ],
            eps = 1e-3
        );
    }
}
