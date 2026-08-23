use super::*;
use rosomaxa::prelude::Float;

pub(crate) fn draw_on_area<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &GsomDrawConfig,
) -> DrawResult<()> {
    area.fill(&WHITE)?;
    if config.points.is_empty() {
        area.draw(&Text::new(
            "No GSOM topology snapshots at this generation",
            (20, 30),
            ("sans-serif", 16).into_font().color(&BLACK),
        ))?;
        return Ok(());
    }

    let mut areas = area.split_evenly((3, 1));
    draw_counts(areas.get_mut(0).unwrap(), &config.points)?;
    draw_ratios(areas.get_mut(1).unwrap(), &config.points)?;
    draw_mse(areas.get_mut(2).unwrap(), &config.points)?;

    Ok(())
}

fn draw_counts<B: DrawingBackend + 'static>(area: &DrawingArea<B, Shift>, points: &[GsomStatePoint]) -> DrawResult<()> {
    let max_generation = points.last().map_or(1, |point| point.generation.saturating_add(1).max(1));
    let max_count = points
        .iter()
        .flat_map(|point| [point.nodes, point.occupied_nodes, point.active_nodes, point.sink_proxies])
        .max()
        .unwrap_or(1)
        .max(1);
    let mut chart = ChartBuilder::on(area)
        .caption("Topology size", ("sans-serif", 13))
        .margin(5)
        .set_label_area_size(LabelAreaPosition::Left, 45)
        .set_label_area_size(LabelAreaPosition::Bottom, 25)
        .build_cartesian_2d(0_usize..max_generation, 0_usize..max_count.saturating_add(1))?;

    chart.configure_mesh().x_desc("generation").y_desc("nodes").draw()?;
    let series = [
        ("map", RED, Box::new(|point: &GsomStatePoint| point.nodes) as Box<dyn Fn(&GsomStatePoint) -> usize>),
        ("occupied", BLUE, Box::new(|point: &GsomStatePoint| point.occupied_nodes)),
        ("recently hit", GREEN, Box::new(|point: &GsomStatePoint| point.active_nodes)),
        ("sink proxies", MAGENTA, Box::new(|point: &GsomStatePoint| point.sink_proxies)),
    ];

    for (label, color, value) in series {
        chart
            .draw_series(LineSeries::new(points.iter().map(|point| (point.generation, value(point))), color))?
            .label(label)
            .legend(move |(x, y)| PathElement::new([(x, y), (x + 14, y)], color));
    }
    chart.configure_series_labels().position(SeriesLabelPosition::UpperLeft).border_style(BLACK).draw()?;

    Ok(())
}

fn draw_ratios<B: DrawingBackend + 'static>(area: &DrawingArea<B, Shift>, points: &[GsomStatePoint]) -> DrawResult<()> {
    let max_generation = points.last().map_or(1, |point| point.generation.saturating_add(1).max(1));
    let mut chart = ChartBuilder::on(area)
        .caption("Map utilization and learning state", ("sans-serif", 13))
        .margin(5)
        .set_label_area_size(LabelAreaPosition::Left, 45)
        .set_label_area_size(LabelAreaPosition::Bottom, 25)
        .build_cartesian_2d(0_usize..max_generation, 0_f64..1_f64)?;

    chart.configure_mesh().x_desc("generation").y_desc("ratio").draw()?;
    let series = [
        ("density", BLUE, Box::new(|point: &GsomStatePoint| point.density) as Box<dyn Fn(&GsomStatePoint) -> Float>),
        ("recent-hit ratio", GREEN, Box::new(|point: &GsomStatePoint| point.active_ratio)),
        ("learning rate", RED, Box::new(|point: &GsomStatePoint| point.learning_rate)),
    ];

    for (label, color, value) in series {
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|point| (point.generation, value(point).clamp(0., 1.) as f64)),
                color,
            ))?
            .label(label)
            .legend(move |(x, y)| PathElement::new([(x, y), (x + 14, y)], color));
    }
    chart.configure_series_labels().position(SeriesLabelPosition::UpperLeft).border_style(BLACK).draw()?;

    Ok(())
}

fn draw_mse<B: DrawingBackend + 'static>(area: &DrawingArea<B, Shift>, points: &[GsomStatePoint]) -> DrawResult<()> {
    let max_generation = points.last().map_or(1, |point| point.generation.saturating_add(1).max(1));
    let (min, max) = points
        .iter()
        .fold((Float::INFINITY, Float::NEG_INFINITY), |(min, max), point| (min.min(point.mse), max.max(point.mse)));
    let padding = if (max - min).abs() <= Float::EPSILON { (min.abs() * 0.05).max(1.) } else { (max - min) * 0.05 };
    let mut chart = ChartBuilder::on(area)
        .caption("Network quantization error", ("sans-serif", 13))
        .margin(5)
        .set_label_area_size(LabelAreaPosition::Left, 45)
        .set_label_area_size(LabelAreaPosition::Bottom, 25)
        .build_cartesian_2d(0_usize..max_generation, (min - padding)..(max + padding))?;

    chart.configure_mesh().x_desc("generation").y_desc("MSE").draw()?;
    chart.draw_series(LineSeries::new(points.iter().map(|point| (point.generation, point.mse)), BLACK))?;

    Ok(())
}
