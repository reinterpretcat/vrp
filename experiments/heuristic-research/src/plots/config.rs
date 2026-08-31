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
    /// Fraction of a combined population plot reserved for this view.
    pub area_ratio: Float,
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
    /// Optional surface function.
    pub surface: Option<Box<dyn Fn(Float, Float) -> Float>>,
    /// Points iterator.
    pub points: Box<dyn Fn() -> Vec<ColoredDataPoint3D>>,
}

/// Specifies drawing configuration for population state.
pub struct PopulationDrawConfig {
    /// Series configuration.
    pub series: PopulationSeries,
    /// Labels of objective fitness planes.
    pub fitness_labels: Vec<String>,
}

/// A population series.
pub enum PopulationSeries {
    /// Unknown (or unimplemented) population type.
    Unknown,
    /// Rosomaxa population type.
    Rosomaxa {
        /// Generation represented by this GSOM snapshot.
        generation: usize,
        /// True when a later generation has no active GSOM and the last exploration snapshot is shown.
        is_stale: bool,
        /// Rows range.
        rows: Range<i32>,
        /// Columns range.
        cols: Range<i32>,
        /// MSE distance.
        mse: Float,
        /// Learning rate.
        learning_rate: Float,
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

/// Specifies drawing configuration for state-specific operator statistics.
#[derive(Default)]
pub struct SearchDrawConfig {
    /// State and checkpoint shown by posterior statistics.
    pub posterior_caption: String,
    /// State and exact generation interval shown by cumulative statistics.
    pub interval_caption: String,
    /// Current effective selection mean with respective operator labels.
    pub posterior: Vec<(String, Float)>,
    /// Exact empirical incumbent-improvement rate with outcome/call counts in the label.
    pub success_rates: Vec<(String, Float)>,
    /// Exact calls of specific operators.
    pub calls: Vec<(String, Float)>,
    /// Exact mean duration in microseconds for specific operators.
    pub durations: Vec<(String, Float)>,
    /// Explanation shown when the selected metric is unavailable.
    pub unavailable: String,
}

/// A series configuration.
pub struct Series2D {
    /// Matrix data.
    pub matrix: MatrixData,
}

/// GSOM topology metrics captured at one generation.
pub struct GsomStatePoint {
    /// Generation.
    pub generation: usize,
    /// Total map nodes.
    pub nodes: usize,
    /// Nodes containing at least one solution.
    pub occupied_nodes: usize,
    /// Nodes hit within the GSOM hit-memory window.
    pub active_nodes: usize,
    /// Map-local fitness sinks used as basin proxies.
    pub sink_proxies: usize,
    /// Fraction of the map bounding box occupied by nodes.
    pub density: Float,
    /// Fraction of nodes hit recently.
    pub active_ratio: Float,
    /// Mean squared map error.
    pub mse: Float,
    /// GSOM learning rate.
    pub learning_rate: Float,
}

/// GSOM topology evolution drawing configuration.
pub struct GsomDrawConfig {
    /// Captured GSOM states.
    pub points: Vec<GsomStatePoint>,
}
