use itertools::izip;

pub fn deltas(input: &[f32], block_size: usize) -> Vec<f32> {
    assert!(block_size >= 5);

    let len = input.len();
    assert!(len % block_size == 0);

    let delta_1 = delta(input, block_size);
    let delta_2 = delta(&delta_1, block_size);

    let mut output = vec![0f32; len * 3];

    let block = block_size * 3;
    for (i, a, b, c) in izip!(
        0..,
        input.chunks_exact(block_size),
        delta_1.chunks_exact(block_size),
        delta_2.chunks_exact(block_size)
    ) {
        output[i * block + 0 * block_size..i * block + 1 * block_size].copy_from_slice(a);
        output[i * block + 1 * block_size..i * block + 2 * block_size].copy_from_slice(b);
        output[i * block + 2 * block_size..i * block + 3 * block_size].copy_from_slice(c);
    }

    output
}

fn delta(input: &[f32], block_size: usize) -> Vec<f32> {
    let mut output = vec![0f32; input.len()];

    let rows = input.len() / block_size;

    let padded = {
        let mut padded = vec![0f32; input.len() + block_size * 4];

        padded[0..block_size].copy_from_slice(&input[0..block_size]);
        padded[block_size..2 * block_size].copy_from_slice(&input[0..block_size]);

        padded[2 * block_size..(2 + rows) * block_size].copy_from_slice(&input);

        padded[(2 + rows) * block_size..(3 + rows) * block_size]
            .copy_from_slice(&input[(rows - 1) * block_size..]);
        padded[(3 + rows) * block_size..].copy_from_slice(&input[(rows - 1) * block_size..]);
        padded
    };

    for row in 0..rows {
        for col in 0..block_size {
            let a = padded[(row + 0) * block_size + col];
            let b = padded[(row + 1) * block_size + col];
            let c = padded[(row + 3) * block_size + col];
            let d = padded[(row + 4) * block_size + col];
            output[row * block_size + col] = round(((c - b) + 2f32 * (d - a)) / 10f32);
        }
    }
    output
}

fn round(value: f32) -> f32 {
    let s = value.log10().ceil().max(0f32) as i32;
    let p = (f32::DIGITS as i32).max(s) - s;
    let n = 10.0_f32.powi(p);
    (value * n).round() / n
}

#[cfg(test)]
mod tests {
    use super::deltas;

    #[test]
    fn test() {
        #[rustfmt::skip]
        let input: Vec<f32> = vec![
            1.0, 2.0, 3.0,  4.0,  5.0, //
            1.0, 3.0, 5.0,  7.0,  9.0, //
            1.0, 4.0, 7.0, 10.0, 13.0, //
            1.0, 5.0, 9.0, 13.0, 17.0, //
            1.0, 6.0, 11.0, 16.0, 21.0, //
        ];

        #[rustfmt::skip]
        let output: Vec<f32> = vec![
            1.0, 2.0,  3.0,  4.0,  5.0, 0.0, 0.5, 1.0, 1.5, 2.0, 0.0,  0.13,  0.26,  0.39,  0.52, //
            1.0, 3.0,  5.0,  7.0,  9.0, 0.0, 0.8, 1.6, 2.4, 3.2, 0.0,  0.11,  0.22,  0.33,  0.44, //
            1.0, 4.0,  7.0, 10.0, 13.0, 0.0, 1.0, 2.0, 3.0, 4.0, 0.0,  0.00,  0.00,  0.00,  0.00, //
            1.0, 5.0,  9.0, 13.0, 17.0, 0.0, 0.8, 1.6, 2.4, 3.2, 0.0, -0.11, -0.22, -0.33, -0.44, //
            1.0, 6.0, 11.0, 16.0, 21.0, 0.0, 0.5, 1.0, 1.5, 2.0, 0.0, -0.13, -0.26, -0.39, -0.52, //
        ];

        assert_eq!(deltas(&input, 5), output);
    }
}
