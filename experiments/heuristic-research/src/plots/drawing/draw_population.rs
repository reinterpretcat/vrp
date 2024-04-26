use super::*;
use itertools::{Itertools, MinMaxResult};
use rosomaxa::prelude::{compare_floats, compare_floats_refs};
use std::cmp::Ordering;

/// Draws rosomaxa population state.
pub(crate) fn draw_on_area<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &PopulationDrawConfig,
) -> DrawResult<()> {
    match &config.series {
        PopulationSeries::Rosomaxa {
            rows,
            cols,
            fitness_matrices,
            mean_distance,
            u_matrix,
            t_matrix,
            l_matrix,
            n_matrix,
            ..
        } => {
            let plots = fitness_matrices.len() + 5;
            let cols_size = plots / 2 + usize::from(plots % 2 == 1);

            let rows = rows.start..(rows.end + 1);
            let cols = cols.start..(cols.end + 1);

            let mut sub_areas = area.split_evenly((2, cols_size));
            // draw series using colored rectangles
            let draw_series2d = |area: &mut DrawingArea<B, Shift>,
                                 caption_fn: &dyn Fn(f64, f64) -> String,
                                 series: &Series2D|
             -> DrawResult<()> {
                let matrix: MatrixData = (series.matrix_fn)();
                let (min, max, size) = match matrix.iter().minmax_by(|(_, &a), (_, &b)| compare_floats(a, b)) {
                    MinMaxResult::OneElement((_, &value)) if compare_floats(value, 0.) != Ordering::Equal => {
                        (value, value, value)
                    }
                    MinMaxResult::MinMax((_, &min), (_, &max)) => (min, max, max - min),
                    _ => (1., 1., 1.),
                };

                let mut chart = ChartBuilder::on(area)
                    .caption(caption_fn(min, max).as_str(), ("sans-serif", 12))
                    .margin(5)
                    .build_cartesian_2d(rows.clone(), cols.clone())?;

                chart.configure_mesh().disable_x_mesh().disable_y_mesh().draw()?;

                chart.draw_series(rows.clone().cartesian_product(cols.clone()).map(|(x, y)| {
                    let points = [(x, y), (x + 1, y + 1)];

                    if let Some(v) = matrix.get(&Coordinate(x, y)).cloned() {
                        Rectangle::new(points, HSLColor(240. / 360. - 240. / 360. * (v - min) / size, 1., 0.7).filled())
                    } else {
                        Rectangle::new(points, WHITE)
                    }
                }))?;

                Ok(())
            };

            // draw series like gradients (but these are not gradients)
            let draw_gradients = |area: &mut DrawingArea<B, Shift>,
                                  _caption: &str,
                                  series: &Vec<Series2D>|
             -> DrawResult<()> {
                let vertical_offset = 21;
                let (w, h) = area.dim_in_pixel();
                let h = h - vertical_offset;

                let x_step = (w as f64 / (rows.len()) as f64).round();
                let y_step = (h as f64 / (cols.len()) as f64).round();

                area.fill(&WHITE)?;

                let get_fitness = |coord: &Coordinate| {
                    (series[0].matrix_fn)().get(coord).cloned().map(|v| {
                        std::iter::once(v)
                            .chain((1..series.len()).map(move |idx| *((series[idx].matrix_fn)().get(coord).unwrap())))
                            .collect::<Vec<_>>()
                    })
                };

                let compare_fitness = |left: &[f64], right: &[f64]| {
                    (left.iter())
                        .zip(right.iter())
                        .map(|(lhs, rhs)| compare_floats_refs(lhs, rhs))
                        .find_or_first(|ord| *ord != Ordering::Equal)
                        .unwrap_or(Ordering::Equal)
                };

                let to_relation = |left: &Coordinate, right: &Coordinate| {
                    get_fitness(left)
                        .zip(get_fitness(right))
                        .map(|(left, right)| compare_fitness(left.as_slice(), right.as_slice()))
                };

                let to_points = |left: &Coordinate, right: &Coordinate| {
                    to_relation(left, right).filter(|ord| *ord == Ordering::Greater).map(|_| {
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
                    })
                };

                let get_neighbours = |x: i32, y: i32| {
                    [Coordinate(x, y + 1), Coordinate(x, y - 1), Coordinate(x + 1, y), Coordinate(x - 1, y)]
                };

                let translate = |x: i32, y: i32| {
                    let x = ((x - rows.start) as f64 * x_step).round() as i32;
                    let x_offset = (x_step / 2.).round() as i32;
                    let x = x + x_offset;

                    let y = y - cols.start;
                    let y = (y as f64 * y_step).round() as i32;
                    let y_offset = (y_step / 2.).round() as i32;
                    let y = (vertical_offset + h) as i32 - (y + y_offset);

                    (x, y)
                };

                // draw arrows
                rows.clone()
                    .cartesian_product(cols.clone())
                    .filter_map(|(x, y)| {
                        let current = Coordinate(x, y);

                        let arrows = get_neighbours(x, y)
                            .map(|coordinate| to_points(&current, &coordinate))
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();

                        if arrows.is_empty() {
                            None
                        } else {
                            Some(((x, y), arrows))
                        }
                    })
                    .flat_map(|(coord, arrows)| arrows.into_iter().map(move |arrow| (coord, arrow)))
                    .try_for_each(|((x, y), (line, arrow))| {
                        let (x, y) = translate(x, y);

                        let figure = EmptyElement::at((x, y))
                            + PathElement::new(line, BLUE)
                            + Polygon::new(arrow.map(|(x, y)| (x + line[1].0, y + line[1].1)), BLUE);

                        area.draw(&figure)
                    })?;

                // draw local optimum markers
                rows.clone()
                    .cartesian_product(cols.clone())
                    .filter(|&(x, y)| (series[0].matrix_fn)().get(&Coordinate(x, y)).is_some())
                    .filter(|&(x, y)| {
                        get_neighbours(x, y)
                            .map(|coordinate| to_relation(&Coordinate(x, y), &coordinate))
                            .into_iter()
                            .flatten()
                            .all(|ord| ord != Ordering::Greater)
                    })
                    .map(|(x, y)| translate(x, y))
                    .try_for_each(|(x, y)| {
                        let size = 12;
                        let coord = (x - size / 2, y - size / 2);
                        let style = ("sans-serif", size).into_font().color(&RED);

                        area.draw(&Text::new("x", coord, style))
                    })?;

                Ok(())
            };

            let get_caption_float = |caption: &str| {
                let caption = caption.to_string();
                move |min: f64, max: f64| format!("{caption} [{min:.2}..{max:.2}]")
            };
            let get_caption_usize = |caption: &str| {
                let caption = caption.to_string();
                move |min: f64, max: f64| format!("{} [{}..{}]", caption, min as usize, max as usize)
            };

            let len = fitness_matrices.len();

            draw_series2d(sub_areas.get_mut(len).unwrap(), &get_caption_float("u dist"), u_matrix)?;
            draw_gradients(sub_areas.get_mut(len + 1).unwrap(), "grads", fitness_matrices)?;
            draw_series2d(sub_areas.get_mut(len + 2).unwrap(), &get_caption_usize("total hits"), t_matrix)?;
            draw_series2d(sub_areas.get_mut(len + 3).unwrap(), &get_caption_usize("last hits"), l_matrix)?;
            draw_series2d(
                sub_areas.get_mut(len + 4).unwrap(),
                &get_caption_float(format!("n dist ({:.2})", *mean_distance).as_str()),
                n_matrix,
            )?;

            fitness_matrices.iter().enumerate().try_for_each(|(idx, objective)| {
                let caption = format!("objective {idx}");
                draw_series2d(sub_areas.get_mut(idx).unwrap(), &get_caption_float(caption.as_str()), objective)
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
    pub fn get_points(&self, _aspect: f64) -> [(i32, i32); 3] {
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
