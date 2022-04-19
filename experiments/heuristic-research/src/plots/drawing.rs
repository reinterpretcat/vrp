use super::DrawResult;
use crate::plots::*;
use crate::DataPoint3D;
use itertools::{Itertools, MinMaxResult};
use plotters::coord::Shift;
use rosomaxa::algorithms::gsom::Coordinate;
use rosomaxa::utils::compare_floats;
use std::cmp::Ordering;
use std::ops::Deref;

/// Draws chart on canvas according to the drawing configs.
pub fn draw<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    solution_config: &SolutionDrawConfig,
    population_config: &PopulationDrawConfig,
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    match &population_config.series {
        PopulationSeries::Unknown => {
            draw_solution(&area, solution_config)?;
        }
        PopulationSeries::Rosomaxa { .. } => {
            let (left, right) = area.split_horizontally(500);
            draw_solution(&left, solution_config)?;
            draw_population(&right, population_config)?;
        }
    }

    area.present()?;

    Ok(())
}

fn draw_solution<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    solution_config: &SolutionDrawConfig,
) -> DrawResult<()> {
    let x_axis = (solution_config.axes.x.0.start..solution_config.axes.x.0.end).step(solution_config.axes.x.1);
    let z_axis = (solution_config.axes.z.0.start..solution_config.axes.z.0.end).step(solution_config.axes.z.1);
    let y_axis = solution_config.axes.y.start..solution_config.axes.y.end;

    let mut chart = ChartBuilder::on(area).build_cartesian_3d(x_axis.clone(), y_axis, z_axis.clone())?;

    chart.with_projection(|mut pb| {
        pb.yaw = solution_config.projection.yaw;
        pb.pitch = solution_config.projection.pitch;
        pb.scale = solution_config.projection.scale;
        pb.into_matrix()
    });

    chart.configure_axes().draw()?;

    chart.draw_series(
        SurfaceSeries::xoz(x_axis.values(), z_axis.values(), &solution_config.series.surface)
            .style_func(&|&v| (&HSLColor(240. / 360. - 240. / 360. * v / solution_config.axes.y.end, 1., 0.7)).into()),
    )?;

    let data_points = solution_config.series.points.deref()();

    chart.draw_series(
        data_points
            .iter()
            .filter(|(_, point_type, _)| matches!(point_type, PointType::Circle))
            .map(|(DataPoint3D(x, y, z), _, color)| Circle::new((*x, *y, *z), 3, color)),
    )?;

    chart.draw_series(
        data_points
            .iter()
            .filter(|(_, point_type, _)| matches!(point_type, PointType::Triangle))
            .map(|(DataPoint3D(x, y, z), _, color)| TriangleMarker::new((*x, *y, *z), 5, color)),
    )?;

    Ok(())
}

fn draw_population<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    population_config: &PopulationDrawConfig,
) -> DrawResult<()> {
    match &population_config.series {
        PopulationSeries::Rosomaxa { rows, cols, objective, u_matrix, t_matrix, l_matrix } => {
            let mut sub_areas = area.split_evenly((2, 2));
            assert_eq!(sub_areas.len(), 4);

            let draw_series2d =
                |area: &mut DrawingArea<B, Shift>, caption: &str, series: &Series2D| -> DrawResult<()> {
                    let mut chart = ChartBuilder::on(area)
                        .caption(caption, ("sans-serif", 12))
                        .margin(5)
                        .build_cartesian_2d(rows.clone(), cols.clone())?;

                    chart
                        .configure_mesh()
                        .x_labels(rows.len())
                        .y_labels(cols.len())
                        .disable_x_mesh()
                        .disable_y_mesh()
                        .draw()?;

                    let matrix: MatrixData = series.matrix.deref()();
                    let size = match matrix.iter().minmax_by(|(_, &a), (_, &b)| compare_floats(a, b)) {
                        MinMaxResult::OneElement((_, &value)) if compare_floats(value, 0.) != Ordering::Equal => value,
                        MinMaxResult::MinMax((_, &min), (_, &max)) => max - min,
                        _ => 1.,
                    };

                    chart.draw_series(rows.clone().cartesian_product(cols.clone()).map(|(x, y)| {
                        let points = [(x, y), (x + 1, y + 1)];

                        if let Some(v) = matrix.get(&Coordinate(x, y)).cloned() {
                            Rectangle::new(points, HSLColor(240. / 360. - 240. / 360. * v / size, 1., 0.7).filled())
                        } else {
                            Rectangle::new(points, WHITE)
                        }
                    }))?;

                    Ok(())
                };

            draw_series2d(sub_areas.get_mut(0).unwrap(), "objective value", objective)?;
            draw_series2d(sub_areas.get_mut(1).unwrap(), "unified distance", u_matrix)?;
            draw_series2d(sub_areas.get_mut(2).unwrap(), "total hits", t_matrix)?;
            draw_series2d(sub_areas.get_mut(3).unwrap(), "last hits", l_matrix)?;
        }
        PopulationSeries::Unknown => {}
    };

    Ok(())
}
