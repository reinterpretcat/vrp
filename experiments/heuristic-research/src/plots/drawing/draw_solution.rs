use super::*;

/// Draws solution space.
pub(crate) fn draw_on_area<B: DrawingBackend + 'static>(
    area: &DrawingArea<B, Shift>,
    config: &SolutionDrawConfig,
) -> DrawResult<()> {
    #![allow(clippy::unnecessary_cast)]
    let x_axis = (config.axes.x.0.start..config.axes.x.0.end).step(config.axes.x.1);
    let z_axis = (config.axes.z.0.start..config.axes.z.0.end).step(config.axes.z.1);
    let y_axis = config.axes.y.start..config.axes.y.end;

    let mut chart = ChartBuilder::on(area).caption(config.caption.as_str(), ("sans-serif", 12)).build_cartesian_3d(
        x_axis.clone(),
        y_axis,
        z_axis.clone(),
    )?;

    chart.with_projection(|mut pb| {
        pb.yaw = config.projection.yaw as f64;
        pb.pitch = config.projection.pitch as f64;
        pb.scale = config.projection.scale as f64;
        pb.into_matrix()
    });

    chart.configure_axes().draw()?;

    chart
        .draw_series(SurfaceSeries::xoz(x_axis.values(), z_axis.values(), &config.series.surface).style_func(
            &|&v| (&HSLColor((240. / 360. - 240. / 360. * v / config.axes.y.end) as f64, 1., 0.7)).into(),
        ))?;

    let data_points = (config.series.points)();

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
