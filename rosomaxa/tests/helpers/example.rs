use crate::example::*;
use crate::get_default_population;
use crate::utils::Environment;
use std::sync::Arc;

/// A helper method to create an example of VectorContext.
pub fn create_default_heuristic_context() -> VectorContext {
    create_heuristic_context_with_solutions(vec![], create_rosenbrock_function())
}

/// A helper method to create an example of VectorContext with given solutions and objective function.
pub fn create_heuristic_context_with_solutions(
    solutions: Vec<Vec<f64>>,
    objective_func: VectorFunction,
) -> VectorContext {
    let environment = Arc::new(Environment::default());
    let objective = Arc::new(VectorObjective::new(objective_func));

    let mut population = get_default_population::<VectorContext, _, _>(objective.clone(), environment.clone());

    let solutions = solutions.into_iter().map(|data| VectorSolution::new(data, objective.clone())).collect();
    population.add_all(solutions);

    VectorContext::new(objective, population, environment)
}
