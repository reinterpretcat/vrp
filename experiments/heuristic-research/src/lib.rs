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

#[wasm_bindgen]
pub fn run_experiment() {
    let selection_size = 8;
    let objective_name = "rosenbrock";

    run_solver(objective_name, selection_size)
}
