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

/// Result of screen to chart coordinates conversion.
#[wasm_bindgen]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[wasm_bindgen]
impl Chart {
    /// Renders plot for rosenbrock function.
    pub fn rosenbrock(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        drawing::draw(
            canvas,
            &DrawConfig {
                axes: Axes { x: (-2.0..2.0, 0.15), y: (0.0..3610.0), z: (-2.0..2.0, 0.15) },
                projection: Projection { pitch, yaw, scale: 0.8 },
                series: Series {
                    surface: {
                        let fitness_fn = get_fitness_fn_by_name("rosenbrock");
                        Box::new(move |x, z| fitness_fn.deref()(&[x, z]))
                    },
                    points: Box::new(move || get_points(generation)),
                },
            },
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }
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
