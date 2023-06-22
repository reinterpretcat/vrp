use super::*;

/// Draws best known fitness over search progression.
pub fn draw_fitness<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    fitness_config: &FitnessDrawConfig,
) -> DrawResult<()> {
    let fitness = &fitness_config.fitness;
    if fitness.is_empty() {
        return Ok(());
    }

    // get dimensions as amount of objectives, must be the same for all generations
    let dimensions = fitness[0].1.len();
    let generations = fitness.iter().map(|(gen, _)| *gen).max().unwrap();
    let target_idx = fitness_config.target_idx;
    assert!(target_idx < dimensions);

    // get min/max values for each dimension
    let (min_values, max_values) = fitness.iter().fold(
        (vec![f64::INFINITY; dimensions], vec![f64::NEG_INFINITY; dimensions]),
        |(mut min, mut max), (_, row)| {
            assert_eq!(row.len(), dimensions);

            min.iter_mut().zip(row.iter()).for_each(|(entry, &candidate)| *entry = entry.min(candidate));
            max.iter_mut().zip(row.iter()).for_each(|(entry, &candidate)| *entry = entry.max(candidate));

            (min, max)
        },
    );

    area.fill(&WHITE)?;

    let x_min = min_values[target_idx] - 0.05 * min_values[target_idx];
    let x_max = max_values[target_idx] + 0.05 * max_values[target_idx];

    let mut chart = ChartBuilder::on(area)
        .caption("Best fitness", ("sans-serif", (2).percent_height()))
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .margin((1).percent())
        .build_cartesian_2d(0.0..generations as f64, x_min..x_max)?;

    chart.configure_mesh().draw()?;

    (0..dimensions).try_for_each(|dim_idx| {
        let color = Palette99::pick(dim_idx).mix(0.9);
        chart
            .draw_series(LineSeries::new(
                fitness.iter().map(|(generation, fitness)| {
                    let value = fitness[dim_idx];
                    let fitness = if dim_idx == target_idx {
                        value
                    } else {
                        // normalize and scale values inside target objective range
                        let (min_value, max_value) = (min_values[dim_idx], max_values[dim_idx]);
                        let (new_min, new_max) = (min_values[target_idx], max_values[target_idx]);
                        (value - min_value) / (max_value - min_value) * (new_max - new_min) + new_min
                    };

                    (*generation as f64, fitness)
                }),
                &color,
            ))?
            .label(&fitness_config.labels[dim_idx])
            .legend(move |(x, y)| PathElement::new(vec![(x, y - 5), (x + 10, y + 5)], color.filled()));

        DrawResult::Ok(())
    })?;

    chart.configure_series_labels().border_style(BLACK).draw()?;

    Ok(())
}
