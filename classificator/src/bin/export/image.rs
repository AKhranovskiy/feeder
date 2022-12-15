#[cfg(feature = "export-image")]
pub(crate) fn export_image(data: &[f32], kind: &str) {
    use classificator::config::MFCC_N_COEFFS;
    use itertools::Itertools;
    use ordered_float::OrderedFloat;
    use plotters::prelude::{IntoDrawingArea, SVGBackend};
    use plotters::style::RGBColor;
    use crate::util::ensure_dir_exists;

    let (min, max) = match data.iter().minmax_by_key(|&v| OrderedFloat(*v)) {
        itertools::MinMaxResult::NoElements => (0.0, 0.0),
        itertools::MinMaxResult::OneElement(v) => (*v, *v),
        itertools::MinMaxResult::MinMax(a, b) => (*a, *b),
    };

    let mut colors = colorgrad::spectral().colors(MFCC_N_COEFFS);
    colors.reverse();

    let grad = colorgrad::CustomGradient::new()
        .colors(&colors)
        .domain(&[min.into(), max.into()])
        .build()
        .expect("failed to build gradient");

    ensure_dir_exists("plots");

    let hash: u64 = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let data = data
            .iter()
            .cloned()
            .map(OrderedFloat::from)
            .collect::<Vec<_>>();
        Hash::hash_slice(&data, &mut hasher);
        hasher.finish()
    };

    let path = format!("plots/{kind}-{hash:x}.svg");

    assert!(data.len() % MFCC_N_COEFFS == 0, "Invalid data dimension");

    let width = MFCC_N_COEFFS;
    let height = data.len() / MFCC_N_COEFFS;

    println!("PLOT {width}x{height}");

    const PLOT_FRAME_WIDTH: u32 = 2;
    const PLOT_FRAME_HEIGHT: u32 = 10;

    let root = SVGBackend::new(
        &path,
        (
            width as u32 * PLOT_FRAME_WIDTH,
            height as u32 * PLOT_FRAME_HEIGHT,
        ),
    )
    .into_drawing_area();

    let areas = root.split_evenly((height, width));
    for (area, &value) in areas.into_iter().zip(data.iter()) {
        let (r, g, b, _) = grad.at(value as f64).to_linear_rgba_u8();
        let color = RGBColor(r, g, b);

        area.fill(&color).expect("failed to fill area");
    }
}

#[cfg(not(feature = "export-image"))]
pub(crate) fn export_image(_: &[f32], _: &str) {}
