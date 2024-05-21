use super::*;
use plotters::style::full_palette::BLUE_200;
use rosomaxa::prelude::compare_floats_refs;

const TOP_SIZE: usize = 25;

/// Draws search iteration statistics as bar plot.
pub(crate) fn draw_search_iteration<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    fitness_config: &SearchDrawConfig,
) -> DrawResult<()> {
    let (labels, data): (Vec<_>, Vec<_>) = fitness_config.estimations.iter().cloned().unzip();

    draw_bar_plot(area, labels.as_slice(), data.as_slice())
}

/// Draws search best known statistic as bar plot.
pub(crate) fn draw_search_best_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    fitness_config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(area, fitness_config.best.as_slice())
}

/// Draws search durations statistic as bar plot.
pub(crate) fn draw_search_duration_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    fitness_config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(area, fitness_config.durations.as_slice())
}

/// Draws search overall statistic as bar plot.
pub(crate) fn draw_search_overall_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    fitness_config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(area, fitness_config.overall.as_slice())
}

fn draw_search_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    statistics: &[(String, usize)],
) -> DrawResult<()> {
    let mut statistics = statistics.to_vec();

    statistics.sort_by(|(_, a), (_, b)| b.cmp(a));

    let (labels, data): (Vec<String>, Vec<f64>) =
        statistics.into_iter().take(TOP_SIZE).map(|(label, data)| (label, data as f64)).unzip();

    draw_bar_plot(area, labels.as_slice(), data.as_slice())
}

fn draw_bar_plot<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    labels: &[String],
    data: &[f64],
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    let max_x = data.iter().copied().max_by(compare_floats_refs).unwrap_or(1.);
    let max_y = data.len() - 1;
    // TODO: improve font size detection
    let font_size = if max_y < TOP_SIZE { 16 } else { 6 };

    let mut chart = ChartBuilder::on(area)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(0.0..max_x, (0..max_y).into_segmented())?;

    chart.configure_mesh().draw()?;

    chart.draw_series((0..).zip(data.iter()).map(|(y, x)| {
        let mut bar =
            Rectangle::new([(0.0, SegmentValue::Exact(y)), (*x, SegmentValue::Exact(y + 1))], BLUE_200.filled());
        bar.set_margin(2, 2, 0, 0);
        bar
    }))?;

    chart.draw_series((0..).zip(labels.iter()).map(|(y, label)| {
        Text::new(label.clone(), (0.0, SegmentValue::Exact(y + 1)), ("sans-serif", font_size).into_font().color(&BLACK))
    }))?;

    Ok(())
}
