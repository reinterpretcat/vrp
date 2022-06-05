#![allow(clippy::unused_unit)]

#[macro_use]
extern crate lazy_static;

use crate::solver::*;
use rosomaxa::algorithms::gsom::Coordinate;
use std::collections::HashMap;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

mod plots;
pub use self::plots::{draw_plots, Axes};

mod solver;
pub use self::solver::solve_function;

/// Specifies a matrix data type.
pub type MatrixData = HashMap<Coordinate, f64>;

/// Represents a single experiment observation data.
pub enum ObservationData {
    /// Observation for benchmarking function experiment.
    Function(DataPoint3D),
    /// Observation for Vehicle Routing Problem experiment.
    Vrp(DataGraph),
}

lazy_static! {
    /// Keeps track of data used by the solver population.
    static ref EXPERIMENT_DATA: Mutex<ExperimentData> = Mutex::new(ExperimentData::default());
}

/// Runs 3D functions experiment.
#[wasm_bindgen]
pub fn run_function_experiment(function_name: &str, population_type: &str, x: f64, z: f64, generations: usize) {
    let selection_size = 8;
    let logger = Arc::new(|message: &str| {
        web_sys::console::log_1(&message.into());
    });

    solve_function(function_name, population_type, selection_size, vec![x, z], generations, logger)
}

/// Runs VRP experiment.
#[wasm_bindgen]
pub fn run_vrp_experiment(format_type: &str, problem: &str, population_type: &str, generations: usize) {
    let problem = problem.to_string();
    let selection_size = 8;
    let logger = Arc::new(|message: &str| {
        web_sys::console::log_1(&message.into());
    });

    solve_vrp(format_type, problem, population_type, selection_size, generations, logger)
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

/// Gets data graph for given generation (for vrp experiments only).
#[wasm_bindgen]
pub fn get_data_graphs(generation: usize) -> JsValue {
    let graphs: Vec<DataGraph> = EXPERIMENT_DATA
        .lock()
        .unwrap()
        .on_generation
        .get(&generation)
        .map(|(_, data)| data.iter().map(|d| d.into()).collect())
        .unwrap_or_else(|| Vec::new());

    // NOTE merge all edges into one graph
    let graph = if let Some(graph) = graphs.first() {
        DataGraph {
            nodes: graph.nodes.clone(),
            edges: graphs.iter().flat_map(|graph| graph.edges.iter()).cloned().collect(),
        }
    } else {
        DataGraph { nodes: vec![], edges: vec![] }
    };

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serde_json::to_writer_pretty(writer, &graph).expect("cannot serialize graph");

    JsValue::from_str(buffer.as_str())
}
