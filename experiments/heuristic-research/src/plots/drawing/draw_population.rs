use super::*;
use itertools::{Itertools, MinMaxResult};
use rosomaxa::prelude::Float;

/// Draws rosomaxa population state.
pub(crate) fn draw_on_area<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &PopulationDrawConfig,
) -> DrawResult<()> {
    #![allow(clippy::unnecessary_cast)]
    const CAPTION_FONT_SIZE: u32 = 10;

    match &config.series {
        PopulationSeries::Rosomaxa {
            generation,
            is_stale,
            rows,
            cols,
            fitness_matrices,
            mse,
            learning_rate,
            u_matrix,
            t_matrix,
            l_matrix,
            m_matrix,
        } => {
            let plots = fitness_matrices.len() + 5;
            let width = area.dim_in_pixel().0;
            let cols_size = if plots > 6 && width < 850 {
                3
            } else if width < 700 {
                2
            } else {
                plots.div_ceil(2)
            };
            let rows_size = plots.div_ceil(cols_size);

            let rows = rows.clone();
            let cols = cols.clone();

            let mut sub_areas = area.split_evenly((rows_size, cols_size));
            // draw series using colored rectangles
            let draw_series2d = |area: &mut DrawingArea<B, Shift>,
                                 caption_fn: &dyn Fn(Float, Float) -> String,
                                 series: &Series2D|
             -> DrawResult<()> {
                let matrix = &series.matrix;
                let (min, max) = match matrix.iter().minmax_by(|(_, a), (_, b)| a.total_cmp(b)) {
                    MinMaxResult::OneElement((_, &value)) => (value, value),
                    MinMaxResult::MinMax((_, &min), (_, &max)) => (min, max),
                    _ => (0., 0.),
                };
                let span = max - min;

                let mut chart = ChartBuilder::on(area)
                    .caption(caption_fn(min, max).as_str(), ("sans-serif", CAPTION_FONT_SIZE))
                    .margin(5)
                    .build_cartesian_2d(rows.clone(), cols.clone())?;

                chart.configure_mesh().disable_x_mesh().disable_y_mesh().draw()?;

                chart.draw_series(rows.clone().cartesian_product(cols.clone()).map(|(x, y)| {
                    let points = [(x, y), (x + 1, y + 1)];

                    if let Some(v) = matrix.get(&Coordinate(x, y)).copied() {
                        let ratio = if span.abs() <= Float::EPSILON { 0.5 } else { (v - min) / span };
                        Rectangle::new(
                            points,
                            HSLColor((240. / 360. - 240. / 360. * ratio.clamp(0., 1.)) as f64, 1., 0.7).filled(),
                        )
                    } else {
                        Rectangle::new(points, WHITE)
                    }
                }))?;

                Ok(())
            };

            // Draw the discrete fitness watershed induced by occupied cardinal neighbours.
            let draw_basins = |area: &mut DrawingArea<B, Shift>, series: &Vec<Series2D>| -> DrawResult<()> {
                const PERSISTENT_BASIN_SIZE: usize = 8;
                let vertical_offset = 18;
                let (w, h) = area.dim_in_pixel();
                let h = h - vertical_offset;

                let x_step = (w as Float / (rows.len()) as Float).round();
                let y_step = (h as Float / (cols.len()) as Float).round();

                area.fill(&WHITE)?;

                if series.is_empty() {
                    return Ok(());
                }

                let matrices = series.iter().map(|series| &series.matrix).collect::<Vec<_>>();
                let basins = get_persistent_fitness_basins(matrices.as_slice(), PERSISTENT_BASIN_SIZE);
                let max_depth = basins.depth_by_coordinate.values().copied().max().unwrap_or_default().max(1);
                let caption =
                    format!("persistent basins {}→{} · red x = retained", basins.raw_sinks.len(), basins.sinks.len());
                area.draw(&Text::new(caption, (5, 12), ("sans-serif", CAPTION_FONT_SIZE).into_font().color(&BLACK)))?;

                let to_points = |left: &Coordinate, right: &Coordinate| {
                    let x_step = x_step.round() as i32;
                    let y_step = y_step.round() as i32;

                    let (direction, line) = match (left.0 - right.0, left.1 - right.1) {
                        (0, 1) => (ArrowDirection::Bottom, [(0, 0), (0, y_step)]),
                        (0, -1) => (ArrowDirection::Top, [(0, 0), (0, -y_step)]),
                        (1, 0) => (ArrowDirection::Left, [(0, 0), (-x_step, 0)]),
                        (-1, 0) => (ArrowDirection::Right, [(0, 0), (x_step, 0)]),
                        _ => unreachable!(),
                    };
                    (line, direction.get_points(1.))
                };

                let translate = |x: i32, y: i32| {
                    let x = ((x - rows.start) as Float * x_step).round() as i32;
                    let x_offset = (x_step / 2.).round() as i32;
                    let x = x + x_offset;

                    let y = y - cols.start;
                    let y = (y as Float * y_step).round() as i32;
                    let y_offset = (y_step / 2.).round() as i32;
                    let y = (vertical_offset + h) as i32 - (y + y_offset);

                    (x, y)
                };

                // Basin hue identifies the sink; lighter cells are farther uphill from it.
                basins.sink_by_coordinate.iter().try_for_each(|(coordinate, sink)| {
                    let basin_idx = basins.sinks.binary_search(sink).expect("basin sink is indexed");
                    let depth = basins.depth_by_coordinate[coordinate];
                    let hue = ((basin_idx * 137) % 360) as f64 / 360.;
                    let lightness = 0.78 + 0.16 * depth as f64 / max_depth as f64;
                    let (x, y) = translate(coordinate.0, coordinate.1);
                    let half_x = (x_step / 2.).round() as i32;
                    let half_y = (y_step / 2.).round() as i32;

                    area.draw(&Rectangle::new(
                        [(x - half_x, y - half_y), (x + half_x, y + half_y)],
                        HSLColor(hue, 0.55, lightness).filled(),
                    ))
                })?;

                // One arrow per node follows its steepest lexicographic descent.
                basins.next.iter().filter(|(coordinate, next)| coordinate != next).try_for_each(
                    |(coordinate, next)| {
                        let (line, arrow) = to_points(coordinate, next);
                        let (x, y) = translate(coordinate.0, coordinate.1);

                        let figure = EmptyElement::at((x, y))
                            + PathElement::new(line, BLUE)
                            + Polygon::new(arrow.map(|(x, y)| (x + line[1].0, y + line[1].1)), BLUE);

                        area.draw(&figure)
                    },
                )?;

                // Mark the unique sink of each equal-fitness plateau.
                basins.sinks.iter().map(|coordinate| translate(coordinate.0, coordinate.1)).try_for_each(
                    |(x, y)| {
                        let size = 12;
                        let coord = (x - size / 2, y - size / 2);
                        let style = ("sans-serif", size).into_font().color(&RED);

                        area.draw(&Text::new("x", coord, style))
                    },
                )?;

                Ok(())
            };

            let get_caption_float = |caption: &str| {
                let caption = caption.to_string();
                move |min: Float, max: Float| format!("{caption} [{min:.2}..{max:.2}]")
            };
            let get_caption_usize = |caption: &str| {
                let caption = caption.to_string();
                move |min: Float, max: Float| format!("{} [{}..{}]", caption, min as usize, max as usize)
            };

            let len = fitness_matrices.len();

            let snapshot =
                if *is_stale { format!(" · gen {generation}, inactive") } else { format!(" · gen {generation}") };
            draw_series2d(
                sub_areas.get_mut(len).unwrap(),
                &get_caption_float(format!("map distance{snapshot}").as_str()),
                u_matrix,
            )?;
            draw_basins(sub_areas.get_mut(len + 1).unwrap(), fitness_matrices)?;
            draw_series2d(sub_areas.get_mut(len + 2).unwrap(), &get_caption_usize("total hits"), t_matrix)?;
            draw_series2d(sub_areas.get_mut(len + 3).unwrap(), &get_caption_usize("recent hits"), l_matrix)?;
            draw_series2d(
                sub_areas.get_mut(len + 4).unwrap(),
                &get_caption_float(format!("node error · MSE {:.2} · lr {:.3}", *mse, *learning_rate).as_str()),
                m_matrix,
            )?;

            fitness_matrices.iter().enumerate().try_for_each(|(idx, objective)| {
                let label = config.fitness_labels.get(idx).map(String::as_str).unwrap_or("objective");
                draw_series2d(sub_areas.get_mut(idx).unwrap(), &get_caption_float(label), objective)
            })?;
        }
        PopulationSeries::Unknown => {}
    };

    Ok(())
}

enum ArrowDirection {
    Top,
    Bottom,
    Right,
    Left,
}

impl ArrowDirection {
    pub fn get_points(&self, _aspect: Float) -> [(i32, i32); 3] {
        // TODO translate x and y if aspect ratio != 1
        let data = [(-2, 8), (0, 0), (2, 8)];

        let rotate_fn = |angle: f32, vec: (i32, i32)| -> (i32, i32) {
            let angle = angle * (std::f32::consts::PI / 180.);

            let cos = angle.cos().round() as i32;
            let sin = angle.sin().round() as i32;

            (vec.0 * cos - vec.1 * sin, vec.0 * sin + vec.1 * cos)
        };

        let rotate_triangle_fn =
            |angle: f32| [rotate_fn(angle, data[0]), rotate_fn(angle, data[1]), rotate_fn(angle, data[2])];

        match self {
            ArrowDirection::Top => rotate_triangle_fn(0.),
            ArrowDirection::Bottom => rotate_triangle_fn(180.),
            ArrowDirection::Right => rotate_triangle_fn(90.),
            ArrowDirection::Left => rotate_triangle_fn(-90.),
        }
    }
}
