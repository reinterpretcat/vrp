use super::*;
use rosomaxa::evolution::TelemetryMode;
use rosomaxa::example::*;
use std::ops::Deref;

mod objectives;
pub use self::objectives::*;

/// Specifies a data point type for 3D chart.
#[derive(Clone)]
pub struct DataPoint3D(pub f64, pub f64, pub f64);

/// Runs the solver to minimize objective function with given name.
pub fn solve_function(
    function_name: &str,
    population_type: &str,
    selection_size: usize,
    init_solution: Vec<f64>,
    generations: usize,
    logger: InfoLogger,
) {
    let fitness_fn = get_fitness_fn_by_name(function_name);
    let random = Arc::new(DefaultRandom::default());

    let noise_op = VectorHeuristicOperatorMode::JustNoise(Noise::new(1., (-0.1, 0.1), random));
    let delta_op = VectorHeuristicOperatorMode::JustDelta(-0.1..0.1);
    let delta_power_op = VectorHeuristicOperatorMode::JustDelta(-0.5..0.5);

    let (solutions, _) = Solver::default()
        .with_logger(logger.clone())
        .with_init_solutions(vec![init_solution])
        .with_search_operator(noise_op, "noise", 1.)
        .with_search_operator(delta_op, "delta", 0.2)
        .with_diversify_operator(delta_power_op)
        .with_termination(None, Some(generations), None, None)
        .with_fitness_fn(fitness_fn)
        .with_context_factory(Box::new({
            let logger = logger.clone();
            let population_type = population_type.to_string();
            move |objective, environment| {
                let population =
                    get_population(&population_type, objective.clone(), environment.clone(), selection_size);
                let telemetry_mode =
                    TelemetryMode::OnlyLogging { logger, log_best: 100, log_population: 500, dump_population: false };
                VectorContext::new(objective, population, telemetry_mode, environment)
            }
        }))
        .solve()
        .expect("no solutions");

    let (individual, fitness) = solutions.first().expect("empty solutions");

    logger.deref()(&format!("solution: {individual:?}, fitness: {fitness}"));
}
