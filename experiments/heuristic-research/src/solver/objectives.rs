//! Specifies benchmark functions for metaheuristic testing, see https://en.wikipedia.org/wiki/Test_functions_for_optimization.

use rosomaxa::example::{create_rosenbrock_function, FitnessFn};
use std::sync::Arc;

/// Returns objective function by its name.
pub fn get_fitness_fn_by_name(name: &str) -> FitnessFn {
    match name {
        "rosenbrock" => create_rosenbrock_function(),
        "rastrigin" => create_rastrigin_function(),
        "himmelblau" => create_himmelblau_function(),
        _ => panic!("unknown objective name: `{}`", name),
    }
}

/// Specifies [Rastrigin](https://en.wikipedia.org/wiki/Rastrigin_function) function.
/// xi ∈ [-5.12, 5.12]
fn create_rastrigin_function() -> FitnessFn {
    Arc::new(|input| {
        let a = 10.;
        input
            .iter()
            .fold(a * input.len() as f64, |acc, &item| acc + item * item - a * (2. * std::f64::consts::PI * item).cos())
    })
}

/// Specifies [Himmelblau](https://en.wikipedia.org/wiki/Himmelblau%27s_function) function.
fn create_himmelblau_function() -> FitnessFn {
    Arc::new(|input| {
        assert_eq!(input.len(), 2);

        let x = *input.first().unwrap();
        let y = *input.last().unwrap();

        let left = x * x + y - 11.;
        let right = x + y * y - 7.;

        left * left + right * right
    })
}
