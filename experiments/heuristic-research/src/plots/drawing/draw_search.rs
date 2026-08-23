use super::*;
use plotters::style::full_palette::BLUE_200;
use rosomaxa::prelude::Float;

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

    let (labels, data): (Vec<String>, Vec<Float>) =
        statistics.into_iter().take(TOP_SIZE).map(|(label, data)| (label, data as Float)).unzip();

    draw_bar_plot(area, labels.as_slice(), data.as_slice())
}

fn draw_bar_plot<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    labels: &[String],
    data: &[Float],
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    if data.is_empty() {
        area.draw(&Text::new(
            "No heuristic telemetry at this generation",
            (20, 30),
            ("sans-serif", 16).into_font().color(&BLACK),
        ))?;
        return Ok(());
    }

    let max_x = data.iter().copied().max_by(|a, b| a.total_cmp(b)).unwrap_or(1.).max(Float::EPSILON) * 1.05;
    let max_y = data.len();

    let mut chart = ChartBuilder::on(area)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 300)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(0.0..max_x, (0..max_y).into_segmented())?;

    chart
        .configure_mesh()
        .disable_y_mesh()
        .y_labels(max_y)
        .label_style(("sans-serif", 14))
        .y_label_formatter(&|position| match position {
            SegmentValue::CenterOf(index) => index
                .checked_add(1)
                .and_then(|index| max_y.checked_sub(index))
                .and_then(|index| labels.get(index))
                .cloned()
                .unwrap_or_default(),
            _ => String::new(),
        })
        .draw()?;

    chart.draw_series(data.iter().rev().enumerate().map(|(y, x)| {
        let mut bar =
            Rectangle::new([(0.0, SegmentValue::Exact(y)), (*x, SegmentValue::Exact(y + 1))], BLUE_200.filled());
        bar.set_margin(2, 2, 0, 0);
        bar
    }))?;

    Ok(())
}
