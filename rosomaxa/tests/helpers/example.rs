use crate::example::*;
use crate::get_default_population;
use crate::utils::Environment;
use std::sync::Arc;

/// Creates multidimensional Rosenbrock function, also referred to as the Valley or Banana function.
/// The function is usually evaluated on the hypercube xi ∈ [-5, 10], for all i = 1, …, d, although
/// it may be restricted to the hypercube xi ∈ [-2.048, 2.048], for all i = 1, …, d.
pub fn create_rosenbrock_function() -> VectorFunction {
    Arc::new(|input| {
        assert!(input.len() > 1);

        input.windows(2).fold(0., |acc, pair| {
            let (x1, x2) = match pair {
                [x1, x2] => (*x1, *x2),
                _ => unreachable!(),
            };

            acc + 100. * (x2 - x1.powi(2)).powi(2) + (x1 - 1.).powi(2)
        })
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Deref;

    #[test]
    pub fn can_create_and_use_rosenbrock_function_2d() {
        let function = create_rosenbrock_function();

        assert_eq!(function.deref()(&[2., 2.]), 401.);
        assert_eq!(function.deref()(&[1., 1.]), 0.);
        assert_eq!(function.deref()(&[0.5, 0.5]), 6.5);
        assert_eq!(function.deref()(&[0., 0.]), 1.);
        assert_eq!(function.deref()(&[-0.5, -0.5]), 58.5);
        assert_eq!(function.deref()(&[-1., -1.]), 404.);
        assert_eq!(function.deref()(&[-2., -2.]), 3609.);
    }
}
