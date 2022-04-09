use rosomaxa::example::{create_rosenbrock_function, FitnessFn};
use std::sync::Arc;

/// Returns objective function by its name.
pub fn get_fitness_fn_by_name(name: &str) -> FitnessFn {
    match name {
        "rosenbrock" => create_rosenbrock_function(),
        "rastrigin" => create_rastrigin_function(),
        _ => panic!("unknown objective name: `{}`", name),
    }
}

fn create_rastrigin_function() -> FitnessFn {
    Arc::new(|input| {
        let a = 10.;
        input
            .iter()
            .fold(a * input.len() as f64, |acc, &item| acc + item * item - a * (2. * std::f64::consts::PI * item).cos())
    })
}
