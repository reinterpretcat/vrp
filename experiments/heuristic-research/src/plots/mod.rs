#![allow(clippy::unused_unit)]

use super::*;
use plotters::prelude::*;
use std::ops::Deref;
use web_sys::HtmlCanvasElement;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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
        draw(canvas, generation, pitch, yaw, axes, "rosenbrock")?;
        Ok(())
    }

    /// Draws plot for rastrigin function.
    pub fn rastrigin(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.12..5.12, 0.2), y: (0.0..80.), z: (-5.12..5.12, 0.2) };
        draw(canvas, generation, pitch, yaw, axes, "rastrigin")?;
        Ok(())
    }

    /// Draws plot for himmelblau function.
    pub fn himmelblau(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.0..5.0, 0.2), y: (0.0..700.), z: (-5.0..5.0, 0.2) };
        draw(canvas, generation, pitch, yaw, axes, "himmelblau")?;
        Ok(())
    }

    /// Draws plot for ackley function.
    pub fn ackley(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.0..5.0, 0.2), y: (0.0..14.), z: (-5.0..5.0, 0.2) };
        draw(canvas, generation, pitch, yaw, axes, "ackley")?;
        Ok(())
    }

    /// Draws plot for matyas function.
    pub fn matyas(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-10.0..10.0, 0.4), y: (0.0..100.), z: (-10.0..10.0, 0.4) };
        draw(canvas, generation, pitch, yaw, axes, "matyas")?;
        Ok(())
    }
}

fn draw(
    canvas: HtmlCanvasElement,
    generation: usize,
    pitch: f64,
    yaw: f64,
    axes: Axes,
    name: &str,
) -> Result<(), String> {
    drawing::draw(
        canvas,
        &SolutionDrawConfig {
            axes,
            projection: Projection { pitch, yaw, scale: 0.8 },
            series: Series3D {
                surface: {
                    let fitness_fn = get_fitness_fn_by_name(name);
                    Box::new(move |x, z| fitness_fn.deref()(&[x, z]))
                },
                points: Box::new(move || get_solution_points(generation)),
            },
        },
        &PopulationDrawConfig {
            axes: Axes { x: (Default::default(), 0.0), y: Default::default(), z: (Default::default(), 0.0) },
            series: get_population_series(generation),
        },
    )
    .map_err(|err| err.to_string())
}

fn get_solution_points(generation: usize) -> Vec<ColoredDataPoint3D> {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .map(|data| {
            let mut data_points = vec![];

            if let Some((_, points)) = data.on_generation.get(&generation) {
                data_points.extend(points.iter().map(|point| (point.clone(), PointType::Circle, BLACK)));
            }

            if let Some(points) = data.on_add.get(&generation) {
                data_points.extend(points.iter().map(|point| (point.clone(), PointType::Triangle, RED)));
            }

            if let Some(points) = data.on_select.get(&generation) {
                data_points.extend(points.iter().map(|point| (point.clone(), PointType::Triangle, BLUE)));
            }

            data_points
        })
        .unwrap_or_else(Vec::new)
}

fn get_population_series(generation: usize) -> PopulationSeries {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| match data.population_state.get(&generation) {
            Some(PopulationState::Rosomaxa { rows, cols, objective, u_matrix, t_matrix, l_matrix }) => {
                let get_series = |matrix: &MatrixData| {
                    let matrix = matrix.clone();
                    Series2D { matrix: Box::new(move || matrix.clone()) }
                };

                Some(PopulationSeries::Rosomaxa {
                    rows: rows.clone(),
                    cols: cols.clone(),
                    objective: get_series(objective),
                    u_matrix: get_series(u_matrix),
                    t_matrix: get_series(t_matrix),
                    l_matrix: get_series(l_matrix),
                })
            }
            _ => None,
        })
        .unwrap_or(PopulationSeries::Unknown)
}
