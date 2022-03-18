use crate::DataPoint;
use plotters::style::RGBColor;
use std::ops::Range;

/// Specifies a data point with color type.
pub type ColoredDataPoint = (DataPoint, PointType, RGBColor);

/// Specifies a data point visualization type.
pub enum PointType {
    /// A circle.
    Circle,
    /// A triangle.
    Triangle,
}

/// A drawing configuration.
pub struct DrawConfig {
    /// Axes configuration
    pub axes: Axes,
    /// Projection configuration.
    pub projection: Projection,
    /// Series configuration.
    pub series: Series,
}

/// An axes configuration.
pub struct Axes {
    /// X axis.
    pub x: (Range<f64>, f64),
    /// Y axis.
    pub y: Range<f64>,
    /// Z axis.
    pub z: (Range<f64>, f64),
}

/// A projection configuration.
pub struct Projection {
    /// Pitch.
    pub pitch: f64,
    /// Yaw.
    pub yaw: f64,
    /// Chart scale.
    pub scale: f64,
}

/// A series configuration
pub struct Series {
    /// Surface function.
    pub surface: Box<dyn Fn(f64, f64) -> f64>,
    /// Points iterator.
    pub points: Box<dyn Fn() -> Vec<ColoredDataPoint>>,
}
