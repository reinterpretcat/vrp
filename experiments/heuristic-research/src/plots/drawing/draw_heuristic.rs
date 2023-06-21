use super::*;

/// Draws heuristic state as bar plot.
pub fn draw_heuristic<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    fitness_config: &HeuristicDrawConfig,
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    let labels = &fitness_config.labels;
    let data = &fitness_config.estimations;

    let max_x = fitness_config.max_estimate;
    let max_y = data.len() - 1;
    // TODO: improve font size detection
    let font_size = if max_y < 20 { 16 } else { 8 };

    let mut chart = ChartBuilder::on(area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption("Heuristic probabilities", ("sans-serif", 16))
        .build_cartesian_2d(0.0..max_x, (0..max_y).into_segmented())?;

    chart.configure_mesh().draw()?;

    chart.draw_series((0..).zip(data.iter()).map(|(y, x)| {
        let mut bar = Rectangle::new([(0.0, SegmentValue::Exact(y)), (*x, SegmentValue::Exact(y + 1))], RED.filled());
        bar.set_margin(2, 2, 0, 0);
        bar
    }))?;

    chart.draw_series((0..).zip(labels.iter()).map(|(y, label)| {
        Text::new(label.clone(), (0.0, SegmentValue::Exact(y + 1)), ("sans-serif", font_size).into_font().color(&BLACK))
    }))?;

    Ok(())
}
