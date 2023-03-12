#![allow(dead_code)]
use ndarray::{Array2, Axis};

pub(crate) fn deltas(input: Array2<f64>) -> Array2<f64> {
    let delta_1 = delta(&input);
    let delta_2 = delta(&delta_1);

    let mut output = input.t().to_owned();
    output.append(Axis(0), delta_1.t()).unwrap();
    output.append(Axis(0), delta_2.t()).unwrap();
    output.t().to_owned()
}

fn delta(input: &Array2<f64>) -> Array2<f64> {
    let mut output = Array2::<f64>::zeros((input.nrows(), input.ncols()));

    for i in 0..input.nrows() {
        let a2 = input.row(i.saturating_sub(2)).to_owned();
        let a1 = input.row(i.saturating_sub(1)).to_owned();
        let b1 = input
            .row(i.saturating_add(1).min(input.nrows() - 1))
            .to_owned();
        let b2 = input
            .row(i.saturating_add(2).min(input.nrows() - 1))
            .to_owned();

        // n = 2, denom = 10
        let delta = ((b1 - a1) + 2.0 * (b2 + a2)) / 10.0;
        output.row_mut(i).assign(&delta);
    }

    output
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_delta() {
        let input = array![
            [1.0, 2.0, 3.0, 4.0, 5.0],
            [1.0, 3.0, 5.0, 7.0, 9.0],
            [1.0, 4.0, 7.0, 10.0, 13.0],
            [1.0, 5.0, 9.0, 13.0, 17.0],
            [1.0, 6.0, 11.0, 16.0, 21.0],
        ];
        assert_eq!(
            delta(&input),
            array![
                [0.4, 1.3, 2.2, 3.1, 4.0],
                [0.4, 1.6, 2.8, 4.0, 5.2],
                [0.4, 1.8, 3.2, 4.6, 6.0],
                [0.4, 2.0, 3.6, 5.2, 6.8],
                [0.4, 2.1, 3.8, 5.5, 7.2]
            ]
        );
    }

    #[test]
    fn test_deltas() {
        let input = array![
            [1.0, 2.0, 3.0, 4.0, 5.0],
            [1.0, 3.0, 5.0, 7.0, 9.0],
            [1.0, 4.0, 7.0, 10.0, 13.0],
            [1.0, 5.0, 9.0, 13.0, 17.0],
            [1.0, 6.0, 11.0, 16.0, 21.0],
        ];
        assert_eq!(
            deltas(input),
            array![
                [
                    1.0,
                    2.0,
                    3.0,
                    4.0,
                    5.0,
                    0.4,
                    1.3,
                    2.2,
                    3.1,
                    4.0,
                    0.16,
                    0.65,
                    1.140_000_000_000_000_1,
                    1.629_999_999_999_999_7,
                    2.12
                ],
                [
                    1.0,
                    3.0,
                    5.0,
                    7.0,
                    9.0,
                    0.4,
                    1.6,
                    2.8,
                    4.0,
                    5.2,
                    0.16,
                    0.71,
                    1.260_000_000_000_000_2,
                    1.81,
                    2.360_000_000_000_000_3
                ],
                [
                    1.0,
                    4.0,
                    7.0,
                    10.0,
                    13.0,
                    0.4,
                    1.8,
                    3.2,
                    4.6,
                    6.0,
                    0.16,
                    0.720_000_000_000_000_1,
                    1.28,
                    1.839_999_999_999_999_9,
                    2.4
                ],
                [
                    1.0,
                    5.0,
                    9.0,
                    13.0,
                    17.0,
                    0.4,
                    2.0,
                    3.6,
                    5.2,
                    6.8,
                    0.16,
                    0.77,
                    1.38,
                    1.989_999_999_999_999_8,
                    2.6
                ],
                [
                    1.0,
                    6.0,
                    11.0,
                    16.0,
                    21.0,
                    0.4,
                    2.1,
                    3.8,
                    5.5,
                    7.2,
                    0.16,
                    0.79,
                    1.42,
                    2.05,
                    2.679_999_999_999_999_7
                ]
            ]
        );
    }
}
