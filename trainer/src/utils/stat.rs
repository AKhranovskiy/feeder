#[derive(Default)]
pub struct RunningAverage {
    sum: f64,
    count: usize,
}

impl RunningAverage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(self, value: f64) -> RunningAverage {
        Self {
            sum: self.sum + value,
            count: self.count + 1,
        }
    }

    #[allow(dead_code)]
    pub fn push_many(self, values: &[f64]) -> RunningAverage {
        let sum = self.sum + values.iter().sum::<f64>();
        let count = self.count + values.len();
        Self { sum, count }
    }

    pub fn avg(&self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.sum / self.count as f64)
        }
    }

    pub fn sum(&self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.sum)
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

pub struct Stats {
    min: f64,
    max: f64,
    last: f64,
    running_avg: RunningAverage,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}
impl Stats {
    pub fn new() -> Self {
        Self {
            min: f64::MAX,
            max: f64::MIN,
            last: f64::NAN,
            running_avg: RunningAverage::new(),
        }
    }

    pub fn min(&self) -> Option<f64> {
        if self.running_avg.count() == 0 {
            None
        } else {
            Some(self.min)
        }
    }
    pub fn max(&self) -> Option<f64> {
        if self.running_avg.count() == 0 {
            None
        } else {
            Some(self.max)
        }
    }

    pub fn avg(&self) -> Option<f64> {
        self.running_avg.avg()
    }

    pub fn sum(&self) -> Option<f64> {
        self.running_avg.sum()
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.running_avg.count()
    }

    pub fn last(&self) -> Option<f64> {
        if self.running_avg.count() == 0 {
            None
        } else {
            Some(self.last)
        }
    }

    pub fn push(self, value: f64) -> Self {
        Self {
            min: self.min.min(value),
            max: self.max.max(value),
            last: value,
            running_avg: self.running_avg.push(value),
        }
    }

    #[allow(dead_code)]
    fn push_many(self, values: &[f64]) -> Stats {
        let mut stats = self;
        for value in values {
            stats = stats.push(*value);
        }
        stats
    }
}

#[cfg(test)]
mod test {
    mod running_avg {
        use super::super::RunningAverage;

        #[test]
        fn test_empty() {
            assert!(RunningAverage::new().avg().is_none())
        }

        #[test]
        fn test_one() {
            assert_eq!(Some(3.1), RunningAverage::new().push(3.1).avg())
        }

        #[test]
        fn test_few() {
            assert_eq!(
                Some(7.8375),
                RunningAverage::new()
                    .push(1.0)
                    .push(24.5)
                    .push(3.1)
                    .push(2.75)
                    .avg()
            )
        }

        #[test]
        fn test_many() {
            assert_eq!(
                Some(7.8375),
                RunningAverage::new()
                    .push_many(&[1.0, 24.5, 3.1, 2.75])
                    .avg()
            )
        }
    }

    mod stats {
        use super::super::Stats;

        #[test]
        fn test_empty() {
            let sut = Stats::new();
            assert!(sut.min().is_none());
            assert!(sut.max().is_none());
            assert!(sut.avg().is_none());
            assert!(sut.sum().is_none());
            assert_eq!(0, sut.count());
        }
        #[test]
        fn test_one() {
            let sut = Stats::new().push(5.0);
            assert_eq!(Some(5.0), sut.min());
            assert_eq!(Some(5.0), sut.max());
            assert_eq!(Some(5.0), sut.avg());
            assert_eq!(Some(5.0), sut.sum());
            assert_eq!(1, sut.count());
        }
        #[test]
        fn test_few() {
            let sut = Stats::new().push(5.0).push(-4.4).push(3.1).push(-2.75);
            assert_eq!(Some(-4.4), sut.min());
            assert_eq!(Some(5.0), sut.max());
            assert_eq!(Some(0.23749999999999993), sut.avg());
            assert_eq!(Some(0.9499999999999997), sut.sum());
            assert_eq!(4, sut.count());
        }
        #[test]
        fn test_many() {
            let sut = Stats::new().push_many(&[5.0, -4.4, 3.1, -2.75]);
            assert_eq!(Some(-4.4), sut.min());
            assert_eq!(Some(5.0), sut.max());
            assert_eq!(Some(0.23749999999999993), sut.avg());
            assert_eq!(Some(0.9499999999999997), sut.sum());
            assert_eq!(4, sut.count());
        }
    }
}
