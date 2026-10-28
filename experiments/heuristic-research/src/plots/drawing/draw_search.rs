use super::*;
use plotters::style::full_palette::BLUE_200;
use rosomaxa::prelude::Float;

/// Draws effective Thompson selection means as a bar plot.
pub(crate) fn draw_search_iteration<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(
        area,
        config.posterior.as_slice(),
        config.posterior_caption.as_str(),
        "effective mean",
        "No posterior telemetry at this generation",
    )
}

/// Draws empirical incumbent-improvement success rates as a bar plot.
pub(crate) fn draw_search_best_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(
        area,
        config.success_rates.as_slice(),
        config.interval_caption.as_str(),
        "success rate",
        config.unavailable.as_str(),
    )
}

/// Draws search durations statistic as bar plot.
pub(crate) fn draw_search_duration_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(
        area,
        config.durations.as_slice(),
        config.interval_caption.as_str(),
        "mean duration (µs)",
        config.unavailable.as_str(),
    )
}

/// Draws search overall statistic as bar plot.
pub(crate) fn draw_search_overall_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &SearchDrawConfig,
) -> DrawResult<()> {
    draw_search_statistics(
        area,
        config.calls.as_slice(),
        config.interval_caption.as_str(),
        "calls",
        "No operator calls in this interval",
    )
}

fn draw_search_statistics<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    statistics: &[(String, Float)],
    caption: &str,
    x_description: &str,
    unavailable: &str,
) -> DrawResult<()> {
    let mut statistics = statistics.to_vec();

    statistics.sort_by(|(_, a), (_, b)| b.total_cmp(a));

    let (labels, data): (Vec<String>, Vec<Float>) = statistics.into_iter().unzip();

    draw_bar_plot(area, labels.as_slice(), data.as_slice(), caption, x_description, unavailable)
}

fn draw_bar_plot<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    labels: &[String],
    data: &[Float],
    caption: &str,
    x_description: &str,
    unavailable: &str,
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    if data.is_empty() {
        area.draw(&Text::new(unavailable, (20, 30), ("sans-serif", 16).into_font().color(&BLACK)))?;
        return Ok(());
    }

    let max_x = data.iter().copied().max_by(|a, b| a.total_cmp(b)).unwrap_or(1.).max(Float::EPSILON) * 1.05;
    let max_y = data.len();

    let mut chart = ChartBuilder::on(area)
        .margin(10)
        .caption(caption, ("sans-serif", 17))
        .set_label_area_size(LabelAreaPosition::Left, 300)
        .set_label_area_size(LabelAreaPosition::Bottom, 50)
        .build_cartesian_2d(0.0..max_x, (0..max_y).into_segmented())?;

    chart
        .configure_mesh()
        .disable_y_mesh()
        .y_labels(max_y)
        .x_desc(x_description)
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
