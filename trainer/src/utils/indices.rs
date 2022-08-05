use rand::seq::SliceRandom;

fn random_indices(max: usize, n: usize) -> Vec<usize> {
    let mut rng = rand::thread_rng();
    let mut indices = (0..max).collect::<Vec<_>>();
    indices.shuffle(&mut rng);
    if n < max {
        indices.resize(max.min(n), 0);
    }
    indices
}

/// Generate random sequence of indices from 0 to either `n` or a sum of all elements of `buckets`.
/// Ensures that every "bucket" gets almost equal number of indices.
/// Each bucket represents a sub-slice of a big array. The method distributes indices over each slice.
pub fn random_bucket_indices(buckets: &[usize], n: usize) -> Vec<usize> {
    assert!(buckets.iter().all(|x| x > &0), "Buckets may not be empty.");

    let number_of_buckets = buckets.len();
    let amount_per_bucket = n / number_of_buckets;

    assert!(amount_per_bucket > 1);

    let mut indices = (0..number_of_buckets).collect::<Vec<_>>();
    indices.sort_unstable_by(|a, b| buckets[*a].cmp(&buckets[*b]));

    let mut filled = vec![0usize; number_of_buckets];
    let mut left = Vec::from(buckets);

    for &i in &indices {
        let bucket_size = buckets[i];
        let can_fill = bucket_size.min(amount_per_bucket);
        filled[i] = can_fill;
        left[i] = bucket_size - can_fill;
    }

    let mut leftover = n - filled.iter().sum::<usize>();
    if leftover > 0 {
        let mut rev_indices = indices.clone();
        rev_indices.reverse();

        loop {
            if leftover == 0 {
                break;
            }
            if left.iter().sum::<usize>() == 0 {
                break;
            }

            for &i in &rev_indices {
                if left[i] > 0 {
                    filled[i] += 1;
                    left[i] -= 1;
                    leftover -= 1;
                }

                if leftover == 0 {
                    break;
                }
                if left.iter().sum::<usize>() == 0 {
                    break;
                }
            }
        }
    }

    let mut index_shift: usize = 0;
    let mut result = (0..number_of_buckets)
        .flat_map(|i| {
            let bucket_size = buckets[i];
            let fill = filled[i];
            // Random within a bucket.
            let indices = random_indices(bucket_size, fill)
                .iter()
                .map(|i| i + index_shift)
                .collect::<Vec<_>>();
            index_shift += bucket_size;
            indices
        })
        .collect::<Vec<_>>();
    // Randomize over all buckets.
    result.shuffle(&mut rand::thread_rng());
    result
}

#[cfg(test)]
mod tests {
    mod groupped_random_selection {

        use super::super::random_bucket_indices;

        #[test]
        fn test_single_exact_bucket() {
            let mut sut = random_bucket_indices(&[10], 10);
            sut.sort_unstable();
            assert_eq!(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9], sut);
        }
        #[test]
        fn test_single_small_bucket() {
            let mut sut = random_bucket_indices(&[5], 10);
            sut.sort_unstable();
            assert_eq!(vec![0, 1, 2, 3, 4], sut);
        }

        #[test]
        fn test_big_buckets() {
            let mut sut = random_bucket_indices(&[100, 200, 300], 10);
            sut.sort_unstable();

            assert!(sut[0..3].iter().all(|n| (0..100).contains(n)));
            assert!(sut[3..6].iter().all(|n| (100..300).contains(n)));
            assert!(sut[6..10].iter().all(|n| (300..600).contains(n)));
        }

        #[test]
        fn test_few_exact_buckets() {
            let mut sut = random_bucket_indices(&[5, 5, 5], 15);
            sut.sort_unstable();

            assert_eq!((0..15).collect::<Vec<_>>(), sut);
        }

        #[test]
        fn test_few_exact_buckets_non_round() {
            let mut sut = random_bucket_indices(&[5, 5, 5], 10);
            sut.sort_unstable();

            assert!(sut[0..3].iter().all(|n| (0..5).contains(n)));
            assert!(sut[3..6].iter().all(|n| (5..10).contains(n)));
            assert!(sut[6..10].iter().all(|n| (10..15).contains(n)));
        }

        #[test]
        fn test_unequal_buckets() {
            let mut sut = random_bucket_indices(&[1, 2, 5], 7);
            sut.sort_unstable();

            assert_eq!(7, sut.len());
            assert!(sut[0..1].iter().all(|n| (0..1).contains(n)));
            assert!(sut[1..3].iter().all(|n| (1..3).contains(n)));
            assert!(sut[3..7].iter().all(|n| (3..8).contains(n)));
        }

        #[test]
        fn test_overfill() {
            let mut sut = random_bucket_indices(&[1, 2, 5], 10);
            sut.sort_unstable();

            assert_eq!(8, sut.len());
            assert!(sut[0..1].iter().all(|n| (0..1).contains(n)));
            assert!(sut[1..3].iter().all(|n| (1..3).contains(n)));
            assert!(sut[3..8].iter().all(|n| (3..8).contains(n)));
        }
    }
}
