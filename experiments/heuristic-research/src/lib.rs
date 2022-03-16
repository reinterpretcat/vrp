use crate::solver::*;
use std::time::Duration;
use wasm_bindgen::prelude::*;

mod plots;
mod solver;

#[wasm_bindgen]
pub fn run_experiment() {
    let bound = 1;
    let delay = Some(Duration::from_secs(1));
    let selection_size = 8;
    let objective_name = "rosenbrock";

    // TODO handle callbacks from receivers with some visualizations
    let (senders, _receivers) = create_channels(bound, delay);

    run_solver(objective_name, selection_size, senders)
}
