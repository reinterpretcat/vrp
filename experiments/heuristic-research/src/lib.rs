#![allow(clippy::unused_unit)]

#[macro_use]
extern crate lazy_static;

use crate::solver::*;
use rosomaxa::prelude::Float;
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::fs::File;
use std::io::BufWriter;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

mod plots;
pub use self::plots::{
    Axes, draw_fitness_plots, draw_population_plots, draw_search_best_statistics_plots,
    draw_search_duration_statistics_plots, draw_search_iteration_plots, draw_search_overall_statistics_plots,
};

mod solver;
pub use self::solver::{solve_function, solve_vrp};

/// Coordinate of the node.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct Coordinate(pub i32, pub i32);

impl Serialize for Coordinate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.0, self.1))
    }
}

impl<'de> Deserialize<'de> for Coordinate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(CoordinateVisitor)
    }
}

struct CoordinateVisitor;

impl Visitor<'_> for CoordinateVisitor {
    type Value = Coordinate;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a colon-separated pair of integers between 0 and 255")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let nums = s.split(':').collect::<Vec<_>>();
        if nums.len() == 2 {
            nums[0].parse().ok().zip(nums[1].parse().ok()).map(|(x, y)| Coordinate(x, y))
        } else {
            None
        }
        .ok_or_else(|| Error::invalid_value(Unexpected::Str(s), &self))
    }
}

/// Specifies a matrix data type.
pub type MatrixData = HashMap<Coordinate, Float>;

/// Represents a single experiment observation data.
#[derive(Serialize, Deserialize)]
pub enum ObservationData {
    /// Observation for benchmarking 3D function experiment.
    Function(DataPoint3D),

    /// Observation for Vehicle Routing Problem experiment.
    Vrp(ShadowState),
}

lazy_static! {
    /// Keeps track of data used by the solver population.
    static ref EXPERIMENT_DATA: Mutex<ExperimentData> = Mutex::new(ExperimentData::default());
}

#[inline]
fn set_panic_hook_once() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();
    SET_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            web_sys::console::error_1(&info.to_string().into());
        }));
    });
}

/// Runs 3D functions experiment.
#[wasm_bindgen]
pub fn run_function_experiment(function_name: &str, population_type: &str, x: Float, z: Float, generations: usize) {
    set_panic_hook_once();
    let selection_size = 8;
    let logger = Arc::new(|message: &str| {
        web_sys::console::log_1(&message.into());
    });

    solve_function(function_name, population_type, selection_size, vec![x, z], generations, logger)
}

/// Runs VRP experiment.
#[wasm_bindgen]
pub fn run_vrp_experiment(format_type: &str, problem: &str, population_type: &str, generations: usize) {
    set_panic_hook_once();
    let problem = problem.to_string();
    let selection_size = 8;
    let logger = Arc::new(|message: &str| {
        web_sys::console::log_1(&message.into());
    });

    solve_vrp(format_type, problem, population_type, selection_size, generations, logger)
}

/// Loads experiment data from json serialized representation.
#[wasm_bindgen]
pub fn load_state(data: &str) -> usize {
    match ExperimentData::try_from(data) {
        Ok(data) => *EXPERIMENT_DATA.lock().unwrap() = data,
        Err(err) => web_sys::console::log_1(&err.into()),
    }

    EXPERIMENT_DATA.lock().unwrap().generation
}

/// Clears experiment data.
#[wasm_bindgen]
pub fn clear() {
    EXPERIMENT_DATA.lock().unwrap().clear()
}

/// Gets current (last) generation.
#[wasm_bindgen]
pub fn get_generation() -> usize {
    EXPERIMENT_DATA.lock().unwrap().generation
}

/// Saves state of experiment data.
pub fn save_state(state_file_path: &str) {
    let file = File::create(state_file_path).expect("cannot create file");
    let experiment_data = EXPERIMENT_DATA.lock().unwrap();

    serde_json::to_writer(BufWriter::new(Box::new(file)), experiment_data.deref())
        .expect("cannot save experiment data");
}
