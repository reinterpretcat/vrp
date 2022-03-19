#[macro_use]
extern crate lazy_static;

use crate::solver::*;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

/// Specifies a data point type.
#[derive(Clone)]
pub struct DataPoint(f64, f64, f64);

mod plots;
mod solver;

lazy_static! {
    /// Keeps track of data used by the solver population.
    static ref EXPERIMENT_DATA: Mutex<ExperimentData> = Mutex::new(ExperimentData::default());
}

/// Runs experiment.
#[wasm_bindgen]
pub fn run_experiment(x: f64, z: f64, generations: usize) {
    let selection_size = 8;
    let function_name = "rosenbrock";

    run_solver(function_name, selection_size, vec![x, z], generations)
}

/// Gets current (last) generation.
#[wasm_bindgen]
pub fn get_generation() -> usize {
    EXPERIMENT_DATA.lock().unwrap().generation
}
