use super::DrawResult;
use crate::plots::*;
use crate::DataPoint3D;
use itertools::Itertools;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use rosomaxa::algorithms::gsom::Coordinate;
use std::ops::Deref;
use web_sys::HtmlCanvasElement;

/// Draws chart on canvas according to the drawing configs.
pub fn draw(
    canvas: HtmlCanvasElement,
    solution_config: &SolutionDrawConfig,
    population_config: &PopulationDrawConfig,
) -> DrawResult<()> {
    let area = CanvasBackend::with_canvas_object(canvas).unwrap().into_drawing_area();
    area.fill(&WHITE)?;

    let (left, right) = area.split_horizontally(500);

    draw_solution(left, solution_config)?;

    draw_population(right, population_config)?;

    Ok(())
}

fn draw_solution(area: DrawingArea<CanvasBackend, Shift>, solution_config: &SolutionDrawConfig) -> DrawResult<()> {
    let x_axis = (solution_config.axes.x.0.start..solution_config.axes.x.0.end).step(solution_config.axes.x.1);
    let z_axis = (solution_config.axes.z.0.start..solution_config.axes.z.0.end).step(solution_config.axes.z.1);
    let y_axis = solution_config.axes.y.start..solution_config.axes.y.end;

    let mut chart = ChartBuilder::on(&area).build_cartesian_3d(x_axis.clone(), y_axis, z_axis.clone())?;

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

fn draw_population(
    area: DrawingArea<CanvasBackend, Shift>,
    population_config: &PopulationDrawConfig,
) -> DrawResult<()> {
    match &population_config.series {
        PopulationSeries::Rosomaxa { rows, cols, objective, u_matrix, t_matrix, l_matrix } => {
            let mut sub_areas = area.split_evenly((2, 2));
            assert_eq!(sub_areas.len(), 4);

            let draw_series2d =
                |area: &mut DrawingArea<CanvasBackend, Shift>, caption: &str, series: &Series2D| -> DrawResult<()> {
                    let mut chart = ChartBuilder::on(area)
                        .caption(caption, ("sans-serif", 12))
                        .build_cartesian_2d(rows.clone(), cols.clone())?;

                    chart
                        .configure_mesh()
                        .x_labels(rows.len())
                        .y_labels(cols.len())
                        .disable_x_mesh()
                        .disable_y_mesh()
                        .draw()?;

                    let matrix = series.matrix.deref()();

                    chart.draw_series(rows.clone().cartesian_product(cols.clone()).map(|(x, y)| {
                        let v = matrix.get(&Coordinate(x, y)).cloned().unwrap_or(0.);

                        Rectangle::new(
                            [(x, y), (x + 1, y + 1)],
                            HSLColor(240. / 360. - 240. / 360. * v / 20., 0.7, 0.1 + 0.4 * v / 20.).filled(),
                        )
                    }))?;

                    Ok(())
                };

            draw_series2d(sub_areas.get_mut(0).unwrap(), "objective", objective)?;
            draw_series2d(sub_areas.get_mut(1).unwrap(), "unified", u_matrix)?;
            draw_series2d(sub_areas.get_mut(2).unwrap(), "total", t_matrix)?;
            draw_series2d(sub_areas.get_mut(3).unwrap(), "last", l_matrix)?;
        }
        PopulationSeries::Unknown => {}
    };

    Ok(())
}
