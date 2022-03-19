use rosomaxa::evolution::TelemetryMode;
use rosomaxa::example::*;
use rosomaxa::get_default_population;
use rosomaxa::prelude::*;
use std::sync::Arc;

mod objectives;
pub use self::objectives::*;

mod proxies;
pub use self::proxies::*;

/// Runs the solver to minimize objective function with given name.
pub fn run_solver(function_name: &str, selection_size: usize, init_solution: Vec<f64>, generations: usize) {
    let fitness_fn = get_fitness_fn_by_name(function_name);
    let logger = Arc::new(|message: &str| {
        web_sys::console::log_1(&message.into());
    });

    let random = Arc::new(DefaultRandom::default());
    let noise_op = VectorHeuristicOperatorMode::JustNoise(Noise::new(1., (-0.1, 0.1), random));

    let _ = Solver::default()
        .use_dynamic_heuristic_only()
        .with_logger(logger.clone())
        .with_telemetry_mode(TelemetryMode::OnlyLogging {
            logger,
            log_best: 10,
            log_population: 100,
            dump_population: false,
        })
        .with_init_solutions(vec![init_solution])
        .with_operator(noise_op, "first", 1.)
        .with_termination(None, Some(generations), None, None)
        .with_fitness_fn(fitness_fn)
        .with_context_factory(Box::new(move |objective, environment| {
            let inner =
                get_default_population::<VectorContext, _, _>(objective.clone(), environment.clone(), selection_size);
            let population = Box::new(ProxyPopulation::new(inner));
            VectorContext::new(objective, population, environment)
        }))
        .solve();
}
