use super::*;
use plotters::style::BLACK;
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
pub struct Chart {
    convert: Box<dyn Fn((i32, i32)) -> Option<(f64, f64)>>,
}

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
                        let objective_func = get_objective_function_by_name("rosenbrock");
                        Box::new(move |x, z| objective_func.deref()(&[x, z]))
                    },
                    points: Box::new(move || Self::get_points(generation)),
                },
            },
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    /// This function can be used to convert screen coordinates to chart coordinates.
    pub fn coord(&self, x: i32, y: i32) -> Option<Point> {
        (self.convert)((x, y)).map(|(x, y)| Point { x, y })
    }

    fn get_points(generation: usize) -> Vec<ColoredDataPoint> {
        EXPERIMENT_DATA
            .lock()
            .ok()
            .and_then(|data| {
                // NOTE use generation arg only if it is not zero
                let generation = if generation > 0 { generation } else { data.generation };

                // TODO use different data with different colors
                data.on_generation
                    .get(&generation)
                    .map(|(_, points)| points.iter().map(|(point, _)| (point.clone(), BLACK)).collect())
            })
            .unwrap_or_else(Vec::new)
    }
}
