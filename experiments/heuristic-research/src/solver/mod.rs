use rosomaxa::example::*;
use rosomaxa::get_default_population;
use rosomaxa::prelude::*;
use std::sync::Arc;

mod objectives;
pub use self::objectives::*;

mod proxies;
pub use self::proxies::*;

/// Runs the solver to minimize objective function with given name.
pub fn run_solver(objective_name: &str, selection_size: usize, init_solution: Vec<f64>, generations: usize) {
    let random = Arc::new(DefaultRandom::default());
    let noise_op = VectorHeuristicOperatorMode::JustNoise(Noise::new(1., (-0.1, 0.1), random));

    let _ = Solver::default()
        .with_init_solutions(vec![init_solution])
        .with_operator(noise_op, "first", 1.)
        .with_termination(None, Some(generations), None, None)
        .with_objective_fun(get_objective_function_by_name(objective_name))
        .with_context_factory(Box::new(move |objective, environment| {
            let inner =
                get_default_population::<VectorContext, _, _>(objective.clone(), environment.clone(), selection_size);
            let population = Box::new(ProxyPopulation::new(inner));
            VectorContext::new(objective, population, environment)
        }))
        .solve();
}
