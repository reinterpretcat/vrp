use super::*;

pub fn draw_solution<B: DrawingBackend + 'static>(
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

    let data_points = (solution_config.series.points)();

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
