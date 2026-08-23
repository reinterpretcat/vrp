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
    Axes, draw_fitness_plots, draw_gsom_statistics_plots, draw_population_plots, draw_search_best_statistics_plots,
    draw_search_duration_statistics_plots, draw_search_iteration_plots, draw_search_overall_statistics_plots,
    draw_vrp_population_plots,
};

mod solver;
pub use self::solver::{FunctionConfig, get_fitness_fn_by_name, get_function_config, solve_function, solve_vrp};

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

    /// Legacy VRP observation retained for saved-state compatibility.
    Vrp(ShadowState),
}

#[derive(Serialize)]
struct ExperimentSummary {
    generation: usize,
    snapshot_generation: usize,
    max_generations: usize,
    recording_interval: usize,
    snapshots: usize,
    phase: String,
    fitness: Vec<Float>,
    population_size: usize,
    gsom_generation: Option<usize>,
    gsom_is_stale: bool,
    gsom_nodes: usize,
    gsom_occupied_nodes: usize,
    gsom_active_nodes: usize,
    gsom_sink_proxies: usize,
    gsom_density: Float,
    gsom_mse: Option<Float>,
    gsom_learning_rate: Option<Float>,
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

/// Returns the visible `[x_min, x_max, z_min, z_max]` domain of a benchmark function.
#[wasm_bindgen]
pub fn get_function_domain(function_name: &str) -> Vec<Float> {
    let config = get_function_config(function_name);
    vec![config.x.start, config.x.end, config.z.start, config.z.end]
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

/// Serializes the current experiment data so it can be transferred from a worker to the UI thread.
#[wasm_bindgen]
pub fn get_experiment_state() -> Result<String, JsValue> {
    let data = EXPERIMENT_DATA.lock().map_err(|_| JsValue::from_str("experiment data lock is poisoned"))?;
    serde_json::to_string(data.deref()).map_err(|err| JsValue::from_str(&format!("cannot serialize experiment: {err}")))
}

/// Returns a compact summary for the requested generation.
#[wasm_bindgen]
pub fn get_experiment_summary(generation: usize) -> Result<String, JsValue> {
    let data = EXPERIMENT_DATA.lock().map_err(|_| JsValue::from_str("experiment data lock is poisoned"))?;
    let (snapshot_generation, current_state) = data
        .population_state
        .range(..=generation)
        .next_back()
        .ok_or_else(|| JsValue::from_str("no experiment snapshot is available"))?;
    let fitness = match current_state {
        PopulationState::Rosomaxa { fitness_values, .. } | PopulationState::Unknown { fitness_values } => {
            fitness_values.clone()
        }
    };
    let phase = data
        .population_phases
        .range(..=generation)
        .next_back()
        .map(|(_, phase)| phase.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let population_size =
        data.population_sizes.range(..=generation).next_back().map(|(_, size)| *size).unwrap_or_default();
    let gsom = data.population_state.range(..=generation).rev().find_map(|(generation, state)| match state {
        PopulationState::Rosomaxa { rows, cols, mse, learning_rate, fitness_matrices, u_matrix, l_matrix, .. } => {
            Some((generation, rows, cols, mse, learning_rate, fitness_matrices, u_matrix, l_matrix))
        }
        PopulationState::Unknown { .. } => None,
    });
    let (
        gsom_generation,
        gsom_nodes,
        gsom_occupied_nodes,
        gsom_active_nodes,
        gsom_sink_proxies,
        gsom_density,
        gsom_mse,
        gsom_learning_rate,
    ) = gsom.map_or(
        (None, 0, 0, 0, 0, 0., None, None),
        |(generation, rows, cols, mse, learning_rate, fitness, nodes, hits)| {
            let node_count = nodes.len();
            let cells =
                ((rows.end - rows.start).max(1) as usize).saturating_mul((cols.end - cols.start).max(1) as usize);
            (
                Some(*generation),
                node_count,
                fitness.first().map_or(0, MatrixData::len),
                hits.values().filter(|hits| **hits > 0.).count(),
                plots::count_fitness_sinks(fitness),
                node_count as Float / cells.max(1) as Float,
                Some(*mse),
                Some(*learning_rate),
            )
        },
    );
    let summary = ExperimentSummary {
        generation,
        snapshot_generation: *snapshot_generation,
        max_generations: data.max_generations.max(data.generation),
        recording_interval: data.recording_interval.max(1),
        snapshots: data.population_state.len(),
        phase,
        fitness,
        population_size,
        gsom_generation,
        gsom_is_stale: gsom_generation.is_some_and(|gsom_generation| gsom_generation < *snapshot_generation),
        gsom_nodes,
        gsom_occupied_nodes,
        gsom_active_nodes,
        gsom_sink_proxies,
        gsom_density,
        gsom_mse,
        gsom_learning_rate,
    };

    serde_json::to_string(&summary).map_err(|err| JsValue::from_str(&format!("cannot serialize summary: {err}")))
}

/// Loads experiment data from json serialized representation.
#[wasm_bindgen]
pub fn load_state(data: &str) -> Result<usize, JsValue> {
    set_panic_hook_once();
    let data = ExperimentData::try_from(data).map_err(|err| JsValue::from_str(&err))?;
    let generation = data.generation;
    *EXPERIMENT_DATA.lock().map_err(|_| JsValue::from_str("experiment data lock is poisoned"))? = data;

    Ok(generation)
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
