#![allow(clippy::unused_unit)]

#[macro_use]
extern crate lazy_static;

use crate::solver::*;
use rosomaxa::algorithms::gsom::Coordinate;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

mod plots;
pub use self::plots::{draw_fitness_plots, draw_population_plots, Axes};

mod solver;
pub use self::solver::{solve_function, solve_vrp};

/// Specifies a matrix data type.
pub type MatrixData = HashMap<Coordinate, f64>;

/// Represents a single experiment observation data.
pub enum ObservationData {
    /// Observation for benchmarking 3D function experiment.
    Function(DataPoint3D),

    /// Observation for Vehicle Routing Problem experiment.
    /// DataGraph contains solution represented as a directed graph, DataPoint3D represents solution
    /// as a point in 3D space where meaning of each dimension depends on problem variant.
    Vrp((DataGraph, DataPoint3D)),
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
