use std::ops::Range;

pub(crate) fn stepped_windows(width: usize, window: usize, step: usize) -> (usize, usize) {
    assert!(window > 0, "Window size must be greater than 0");
    assert!(step > 0, "Step size must be greater than 0");

    if width < window {
        return (0, window - width);
    }

    let steps: f32 = width.saturating_sub(window) as f32 / step as f32;

    let tail = steps.fract() * step as f32;
    assert!(tail.fract() < 1e6);

    (steps.trunc() as usize + 1, (step - tail.trunc() as usize) % step)
}

#[inline(always)]
pub(crate) fn stepped_window_ranges(width: usize, window: usize, step: usize) -> Vec<Range<usize>> {
    let (steps, _) = stepped_windows(width, window, step);
    (0..steps).map(|i| i * step..i * step + window).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stepped_windows_1_1_0() {
        assert_eq!((0, 1), stepped_windows(0, 1, 1));
        assert_eq!((1, 0), stepped_windows(1, 1, 1));
    }

    #[test]
    fn test_stepped_windows_1_1_5() {
        assert_eq!((5, 0), stepped_windows(5, 1, 1));
    }

    #[test]
    fn test_stepped_windows_3_2_10() {
        assert_eq!((4, 1), stepped_windows(10, 3, 2));
        assert_eq!((5, 0), stepped_windows(10 + 1, 3, 2));
    }

    #[test]
    fn test_stepped_windows_5_2_4() {
        assert_eq!((0, 1), stepped_windows(4, 5, 2));
        assert_eq!((1, 0), stepped_windows(4 + 1, 5, 2));
    }

    #[test]
    fn test_stepped_windows_5_3_13() {
        assert_eq!((3, 1), stepped_windows(13, 5, 3));
        assert_eq!((4, 0), stepped_windows(13 + 1, 5, 3));
    }

    #[test]
    fn test_stepped_windows_2_3_10() {
        assert_eq!((3, 1), stepped_windows(10, 2, 3));
        assert_eq!((4, 0), stepped_windows(10 + 1, 2, 3));
    }

    #[test]
    fn test_stepped_windows_441_220_220239() {
        assert_eq!((1000, 202), stepped_windows(220239, 441, 220));
        assert_eq!((1001, 0), stepped_windows(220239 + 202, 441, 220));
    }

    #[test]
    fn test_stepped_window_ranges_1_1_0() {
        assert!(stepped_window_ranges(0, 1, 1).is_empty());
        assert_eq!(vec![0usize..1], stepped_window_ranges(1, 1, 1));
    }

    #[test]
    fn test_stepped_window_rangess_1_1_5() {
        assert_eq!(
            vec![0..1, 1..2, 2..3, 3..4, 4..5],
            stepped_window_ranges(5, 1, 1)
        );
    }

    #[test]
    fn test_stepped_window_ranges_3_2_10() {
        assert_eq!(
            vec![0..3, 2..5, 4..7, 6..9],
            stepped_window_ranges(10, 3, 2)
        );
        assert_eq!(
            vec![0..3, 2..5, 4..7, 6..9, 8..11],
            stepped_window_ranges(11, 3, 2)
        );
    }

    #[test]
    fn test_stepped_window_ranges_5_2_4() {
        assert!(stepped_window_ranges(4, 5, 2).is_empty());
        assert_eq!(vec![0..5], stepped_window_ranges(5, 5, 2));
    }

    #[test]
    fn test_stepped_window_ranges_2_3_10() {
        assert_eq!(vec![0..2, 3..5, 6..8], stepped_window_ranges(10, 2, 3));
        assert_eq!(
            vec![0..2, 3..5, 6..8, 9..11],
            stepped_window_ranges(12, 2, 3)
        );
    }
}
