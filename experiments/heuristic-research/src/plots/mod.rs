#![allow(clippy::unused_unit)]

use super::*;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use std::ops::Deref;
use web_sys::HtmlCanvasElement;

/// Type alias for the result of a drawing function.
pub type DrawResult<T> = Result<T, Box<dyn std::error::Error>>;

mod config;
pub use self::config::*;

mod drawing;
pub use self::drawing::*;

/// Type used on the JS side to convert screen coordinates to chart coordinates.
#[wasm_bindgen]
pub struct Chart {}

#[wasm_bindgen]
impl Chart {
    /// Draws plot for rosenbrock function.
    pub fn rosenbrock(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-2.0..2.0, 0.15), y: (0.0..3610.), z: (-2.0..2.0, 0.15) };
        draw_function_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "rosenbrock")?;
        Ok(())
    }

    /// Draws plot for rastrigin function.
    pub fn rastrigin(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.12..5.12, 0.2), y: (0.0..80.), z: (-5.12..5.12, 0.2) };
        draw_function_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "rastrigin")?;
        Ok(())
    }

    /// Draws plot for himmelblau function.
    pub fn himmelblau(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.0..5.0, 0.2), y: (0.0..700.), z: (-5.0..5.0, 0.2) };
        draw_function_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "himmelblau")?;
        Ok(())
    }

    /// Draws plot for ackley function.
    pub fn ackley(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.0..5.0, 0.2), y: (0.0..14.), z: (-5.0..5.0, 0.2) };
        draw_function_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "ackley")?;
        Ok(())
    }

    /// Draws plot for matyas function.
    pub fn matyas(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-10.0..10.0, 0.4), y: (0.0..100.), z: (-10.0..10.0, 0.4) };
        draw_function_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "matyas")?;
        Ok(())
    }

    /// Draws plot for VRP problem.
    pub fn vrp(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        // TODO find nice way to visualize vrp solutions in 3D plot
        draw(
            get_canvas_drawing_area(canvas),
            PopulationDrawConfig {
                axes: Axes { x: (Default::default(), 0.0), y: Default::default(), z: (Default::default(), 0.0) },
                series: get_population_series(generation),
            },
            if generation == 0 {
                let (max_x, max_y, max_z) = get_axis_sizes();
                Some(SolutionDrawConfig {
                    axes: Axes {
                        x: (0.0..max_x.max(10.), 0.5),
                        y: (0.0..max_y.max(10.)),
                        z: (0.0..max_z.max(10.), 0.5),
                    },
                    projection: Projection { pitch, yaw, scale: 0.8 },
                    series: Series3D {
                        surface: Box::new(move |_x, _z| 0.),
                        points: Box::new(move || get_solution_points(generation)),
                    },
                })
            } else {
                None
            },
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }
}

fn get_canvas_drawing_area(canvas: HtmlCanvasElement) -> DrawingArea<CanvasBackend, Shift> {
    CanvasBackend::with_canvas_object(canvas).unwrap().into_drawing_area()
}

/// Draws plots on given area.
pub fn draw_function_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    pitch: f64,
    yaw: f64,
    axes: Axes,
    name: &str,
) -> Result<(), String> {
    draw(
        area,
        PopulationDrawConfig {
            axes: Axes { x: (Default::default(), 0.0), y: Default::default(), z: (Default::default(), 0.0) },
            series: get_population_series(generation),
        },
        Some(SolutionDrawConfig {
            axes,
            projection: Projection { pitch, yaw, scale: 0.8 },
            series: Series3D {
                surface: {
                    let fitness_fn = {
                        if name != "vrp" {
                            get_fitness_fn_by_name(name)
                        } else {
                            Arc::new(|_: &[f64]| 0.)
                        }
                    };
                    Box::new(move |x, z| fitness_fn.deref()(&[x, z]))
                },
                points: Box::new(move || get_solution_points(generation)),
            },
        }),
    )
    .map_err(|err| err.to_string())
}

fn get_solution_points(generation: usize) -> Vec<ColoredDataPoint3D> {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .map(|data| {
            let mut data_points: Vec<ColoredDataPoint3D> = vec![];

            if let Some((_, points)) = data.on_generation.get(&generation) {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Circle, BLACK)));
            }

            if let Some(points) = data.on_add.get(&generation) {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Triangle, RED)));
            }

            if let Some(points) = data.on_select.get(&generation) {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Triangle, BLUE)));
            }

            data_points
        })
        .unwrap_or_else(Vec::new)
}

fn to_data_point(observations: &[ObservationData]) -> impl Iterator<Item = &DataPoint3D> + '_ {
    observations.iter().filter_map(|o| match o {
        ObservationData::Function(point) => Some(point),
        _ => None,
    })
}

fn get_population_series(generation: usize) -> PopulationSeries {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| match data.population_state.get(&generation) {
            Some(PopulationState::Rosomaxa { rows, cols, objectives, u_matrix, t_matrix, l_matrix }) => {
                let get_series = |matrix: &MatrixData| {
                    let matrix = matrix.clone();
                    Series2D { matrix: Box::new(move || matrix.clone()) }
                };

                Some(PopulationSeries::Rosomaxa {
                    rows: rows.clone(),
                    cols: cols.clone(),
                    objectives: objectives.iter().map(get_series).collect(),
                    u_matrix: get_series(u_matrix),
                    t_matrix: get_series(t_matrix),
                    l_matrix: get_series(l_matrix),
                })
            }
            _ => None,
        })
        .unwrap_or(PopulationSeries::Unknown)
}

fn get_axis_sizes() -> (f64, f64, f64) {
    #[derive(Serialize)]
    struct Axis {
        pub x: f64,
        pub y: f64,
        pub z: f64,
    }

    EXPERIMENT_DATA.lock().unwrap().on_generation.iter().fold((0_f64, 0_f64, 0_f64), |acc, (_, (_, data))| {
        data.iter().fold(acc, |(max_x, max_y, max_z), data| {
            let &DataPoint3D(x, y, z) = match data {
                ObservationData::Function(point) => point,
                ObservationData::Vrp((_, point)) => point,
            };
            (max_x.max(x), max_y.max(y), max_z.max(z))
        })
    })
}
