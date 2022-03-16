use crate::extensions::{create_channels, get_objective_function_by_name, ProxyPopulation};
use rosomaxa::example::*;
use rosomaxa::get_default_population;
use std::time::Duration;
use wasm_bindgen::prelude::*;

mod extensions;
mod plots;

#[wasm_bindgen]
pub fn run_experiment() {
    let bound = 1;
    let delay = Some(Duration::from_secs(1));
    let selection_size = 8;
    let objective_name = "rosenbrock";

    // TODO handle callbacks from receivers with some visualizations
    let (senders, _receivers) = create_channels(bound, delay);
    let _solver = Solver::default()
        .with_objective_fun(get_objective_function_by_name(objective_name))
        .with_context_factory(Box::new(move |objective, environment| {
            let inner =
                get_default_population::<VectorContext, _, _>(objective.clone(), environment.clone(), selection_size);
            let population = Box::new(ProxyPopulation::new(inner, senders));
            VectorContext::new(objective, population, environment)
        }))
        .solve();
}
