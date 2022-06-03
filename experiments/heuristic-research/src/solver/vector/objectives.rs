//! Specifies benchmark functions for metaheuristic testing, see https://en.wikipedia.org/wiki/Test_functions_for_optimization.

#[cfg(test)]
#[path = "../../../tests/unit/solver/vector/objectives_test.rs"]
mod objectives_test;

use rosomaxa::example::{create_rosenbrock_function, FitnessFn};
use std::sync::Arc;

/// Returns objective function by its name.
pub fn get_fitness_fn_by_name(name: &str) -> FitnessFn {
    match name {
        "rosenbrock" => create_rosenbrock_function(),
        "rastrigin" => create_rastrigin_function(),
        "himmelblau" => create_himmelblau_function(),
        "ackley" => create_ackley_function(),
        "matyas" => create_matyas_function(),
        _ => panic!("unknown objective name: `{}`", name),
    }
}

/// Specifies [Rastrigin](https://en.wikipedia.org/wiki/Rastrigin_function) function.
/// This multimodal function is difficult to solve as it presents numerous local minima locations
/// where an optimization algorithm, with poor explorative capability, has high chances of being
/// trapped. The function’s only globally best solution 0 is found at f(i)=[0,0,…,0] within the
/// domain of [-5.12,5.12].
fn create_rastrigin_function() -> FitnessFn {
    Arc::new(|input| {
        let a = 10.;
        input
            .iter()
            .fold(a * input.len() as f64, |acc, &item| acc + item * item - a * (2. * std::f64::consts::PI * item).cos())
    })
}

/// Specifies [Himmelblau](https://en.wikipedia.org/wiki/Himmelblau%27s_function) function.
/// This is a multimodal function. It is usually solved with continuous values in the domain of
/// [-5,5]. The best solution 0 can be found at four locations: f(x * )=[3.2,2.0],
/// f(x * )=[-2.805118,3.131312], f(xi)=[-3.779310,-3.283186], and f(x * )=[3.584428,-1.848126]
/// in 2 dimensional space.
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

/// Specifies [Ackley](https://en.wikipedia.org/wiki/Ackley_function) function.
/// This multimodal function is one of the most commonly used test function for metaheuristic
/// algorithm evaluation. It has numerous local minima but one global optimal solution found in
/// deep narrow basin in the middle. The best solution 0 is found at f(xi)=[0,0,…,0] in domain
/// [-32,32].
fn create_ackley_function() -> FitnessFn {
    Arc::new(|input| {
        let n = input.len() as f64;

        let square_sum = input.iter().fold(0., |acc, &item| acc + item * item);
        let cosine_sum = input.iter().fold(0., |acc, &item| acc + (2. * std::f64::consts::PI * item).cos());

        let fx = -20. * (-0.2 * (square_sum / n).sqrt()).exp();
        let fx = fx - (cosine_sum / n).exp();

        fx + std::f64::consts::E + 20.
    })
}

/// Specifies Matyas function.
/// The best solution 0 is found at f(i)=[0,0] in domain [-10,10].
fn create_matyas_function() -> FitnessFn {
    Arc::new(|input| {
        assert_eq!(input.len(), 2);

        let x = *input.first().unwrap();
        let y = *input.last().unwrap();

        0.26 * (x * x + y * y) - 0.48 * x * y
    })
}
