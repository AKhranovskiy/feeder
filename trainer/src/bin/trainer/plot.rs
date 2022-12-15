#[cfg(feature = "plot")]
pub(crate) fn plot(image: &tch::Tensor) {
    use plotters::prelude::{IntoDrawingArea, SVGBackend};
    use plotters::style::RGBColor;
    use tch::IndexOp;

    let min: f32 = image.min().into();
    let max: f32 = image.max().into();

    let size = image.size();
    let width = size[1] as usize;
    let height = size[2] as usize;

    let mut colors = colorgrad::spectral().colors(width);
    colors.reverse();

    let grad = colorgrad::CustomGradient::new()
        .colors(&colors)
        .domain(&[min.into(), max.into()])
        .build()
        .expect("failed to build gradient");

    let path = "image.svg";

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
    for (index, area) in areas.into_iter().enumerate() {
        let row = index / width;
        let col = index % width;
        let value: f64 = image.i(0).i(col as i64).i(row as i64).into();

        let (r, g, b, _) = grad.at(value).to_linear_rgba_u8();
        let color = RGBColor(r, g, b);

        area.fill(&color).expect("failed to fill area");
    }
}

#[cfg(not(feature = "plot"))]
pub(crate) fn plot(_: &tch::Tensor) {}
