use crate::example::*;
use crate::utils::{Environment, Float};
use crate::{get_default_population, get_default_selection_size, TelemetryMode};
use std::sync::Arc;

/// Creates an example objective.
pub fn create_example_objective() -> Arc<VectorObjective> {
    let fitness_fn = create_rosenbrock_function();
    let weight_fn = Arc::new(|data: &[Float]| data.to_vec());

    Arc::new(VectorObjective::new(fitness_fn, weight_fn))
}

/// A helper method to create an example of VectorContext.
pub fn create_default_heuristic_context() -> VectorContext {
    create_heuristic_context_with_solutions(vec![])
}

/// A helper method to create an example of VectorContext with given solutions and objective function.
pub fn create_heuristic_context_with_solutions(solutions: Vec<Vec<Float>>) -> VectorContext {
    let environment = Arc::new(Environment::default());
    let objective = create_example_objective();
    let selection_size = get_default_selection_size(environment.as_ref());

    let mut population =
        get_default_population(objective.clone(), VectorRosomaxaContext, environment.clone(), selection_size);

    let solutions = solutions
        .into_iter()
        .map(|data| {
            let fitness = (objective.fitness_fn)(data.as_slice());
            let weights = (objective.weight_fn)(data.as_slice());
            VectorSolution::new(data, fitness, weights)
        })
        .collect();
    population.add_all(solutions);

    VectorContext::new(objective, population, TelemetryMode::None, environment)
}
