use super::DrawResult;
use crate::plots::DrawConfig;
use crate::DataPoint;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use std::ops::Deref;
use web_sys::HtmlCanvasElement;

/// Draws chart on canvas according to the drawing config.
pub fn draw(canvas: HtmlCanvasElement, config: &DrawConfig) -> DrawResult<()> {
    let area = CanvasBackend::with_canvas_object(canvas).unwrap().into_drawing_area();
    area.fill(&WHITE)?;

    let x_axis = (config.axes.x.0.start..config.axes.x.0.end).step(config.axes.x.1);
    let z_axis = (config.axes.z.0.start..config.axes.z.0.end).step(config.axes.z.1);
    let y_axis = config.axes.y.start..config.axes.y.end;

    let mut chart = ChartBuilder::on(&area).build_cartesian_3d(x_axis.clone(), y_axis, z_axis.clone())?;

    chart.with_projection(|mut pb| {
        pb.yaw = config.projection.yaw;
        pb.pitch = config.projection.pitch;
        pb.scale = config.projection.scale;
        pb.into_matrix()
    });

    chart.configure_axes().draw()?;

    chart.draw_series(
        SurfaceSeries::xoz(x_axis.values(), z_axis.values(), &config.series.surface)
            .style_func(&|&v| (&HSLColor(240.0 / 360.0 - 240.0 / 360.0 * v / config.axes.y.end, 1.0, 0.7)).into()),
    )?;

    chart.draw_series(
        config.series.points.deref()().into_iter().map(|(DataPoint(x, y, z), color)| Circle::new((x, y, z), 3, color)),
    )?;

    Ok(())
}
