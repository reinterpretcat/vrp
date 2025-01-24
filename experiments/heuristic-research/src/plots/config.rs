use crate::{DataPoint3D, MatrixData};
use plotters::style::RGBColor;
use rosomaxa::prelude::Float;
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
    pub pitch: Float,
    /// Yaw.
    pub yaw: Float,
    /// Chart scale.
    pub scale: Float,
}

/// A drawing configuration for solution space visualization.
pub struct SolutionDrawConfig {
    /// Chart caption.
    pub caption: String,
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
    pub x: (Range<Float>, Float),
    /// Y axis.
    pub y: Range<Float>,
    /// Z axis.
    pub z: (Range<Float>, Float),
}

/// A series configuration.
pub struct Series3D {
    /// Surface function.
    pub surface: Box<dyn Fn(Float, Float) -> Float>,
    /// Points iterator.
    pub points: Box<dyn Fn() -> Vec<ColoredDataPoint3D>>,
}

/// Specifies drawing configuration for population state.
pub struct PopulationDrawConfig {
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
        /// MSE distance.
        mse: Float,
        /// Best fitness values.
        fitness_values: Vec<Float>,
        /// Objective values chart series.
        fitness_matrices: Vec<Series2D>,
        /// U-matrix values chart series.
        u_matrix: Series2D,
        /// T-matrix values chart series.
        t_matrix: Series2D,
        /// L-matrix values chart series.
        l_matrix: Series2D,
        /// MSE values chart series.
        m_matrix: Series2D,
    },
}

/// Specifies drawing configuration for best fitness.
pub struct FitnessDrawConfig {
    /// Fitness labels.
    pub labels: Vec<String>,
    /// Objective values for each generation.
    pub fitness: Vec<(usize, Vec<Float>)>,
    /// The most variable objective to be used to initialize axis.
    /// Typically, it is the cost (or distance/duration) minimization.
    pub target_idx: usize,
}

/// /// Specifies drawing configuration for search results.
#[derive(Default)]
pub struct SearchDrawConfig {
    /// Actual estimations with respective labels.
    pub estimations: Vec<(String, Float)>,
    /// Number of discovered best known solutions with the respective label.
    pub best: Vec<(String, usize)>,
    /// Overall calls of specific heuristics.
    pub overall: Vec<(String, usize)>,
    /// Actual durations with respective labels.
    pub durations: Vec<(String, usize)>,
}

/// A series configuration.
pub struct Series2D {
    /// A matrix data receiver function.
    pub matrix_fn: Box<dyn Fn() -> MatrixData>,
}
