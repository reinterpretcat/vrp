use crate::{DataPoint3D, MatrixData};
use plotters::style::RGBColor;
use std::ops::Range;

/// Specifies a data point with color type.
pub type ColoredDataPoint3D = (DataPoint3D, PointType, RGBColor);

/// Specifies a data point visualization type.
pub enum PointType {
    /// A circle.
    Circle,
    /// A triangle.
    Triangle,
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

/// A drawing configuration for solution space visualization.
pub struct SolutionDrawConfig {
    /// Axes configuration
    pub axes: Axes,
    /// Projection configuration.
    pub projection: Projection,
    /// Series configuration.
    pub series: Series3D,
}

/// A 3D axes configuration.
pub struct Axes {
    /// X axis.
    pub x: (Range<f64>, f64),
    /// Y axis.
    pub y: Range<f64>,
    /// Z axis.
    pub z: (Range<f64>, f64),
}

/// A series configuration.
pub struct Series3D {
    /// Surface function.
    pub surface: Box<dyn Fn(f64, f64) -> f64>,
    /// Points iterator.
    pub points: Box<dyn Fn() -> Vec<ColoredDataPoint3D>>,
}

/// Specifies drawing configuration for population state.
pub struct PopulationDrawConfig {
    /// Axes configuration.
    pub axes: Axes,
    /// Series configuration.
    pub series: PopulationSeries,
}

/// A population series.
pub enum PopulationSeries {
    /// Unknown (or unimplemented) population type.
    Unknown,
    /// Rosomaxa population type.
    Rosomaxa {
        /// Rows range.
        rows: Range<i32>,
        /// Columns range.
        cols: Range<i32>,
        /// Objective values chart series.
        fitness: Vec<Series2D>,
        /// Mean node distance.
        mean_distance: f64,
        /// U-matrix values chart series.
        u_matrix: Series2D,
        /// T-matrix values chart series.
        t_matrix: Series2D,
        /// L-matrix values chart series.
        l_matrix: Series2D,
        /// Node distance values chart series.
        n_matrix: Series2D,
    },
}

/// A series configuration.
pub struct Series2D {
    /// A matrix data.
    pub matrix: Box<dyn Fn() -> MatrixData>,
}
