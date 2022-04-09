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
        draw(
            canvas,
            generation,
            pitch,
            yaw,
            Axes { x: (-2.0..2.0, 0.15), y: (0.0..3610.), z: (-2.0..2.0, 0.15) },
            "rosenbrock",
        )?;
        Ok(())
    }

    /// Draws plot for rastrigin function.
    pub fn rastrigin(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        draw(
            canvas,
            generation,
            pitch,
            yaw,
            Axes { x: (-5.12..5.12, 0.2), y: (0.0..80.), z: (-5.12..5.12, 0.2) },
            "rastrigin",
        )?;
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
        &DrawConfig {
            axes,
            projection: Projection { pitch, yaw, scale: 0.8 },
            series: Series {
                surface: {
                    let fitness_fn = get_fitness_fn_by_name(name);
                    Box::new(move |x, z| fitness_fn.deref()(&[x, z]))
                },
                points: Box::new(move || get_points(generation)),
            },
        },
    )
    .map_err(|err| err.to_string())
}

fn get_points(generation: usize) -> Vec<ColoredDataPoint> {
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
