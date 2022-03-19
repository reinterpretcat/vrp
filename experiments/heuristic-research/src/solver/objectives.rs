use rosomaxa::example::{create_rosenbrock_function, FitnessFn};

/// Returns objective function by its name.
pub fn get_fitness_fn_by_name(name: &str) -> FitnessFn {
    match name {
        "rosenbrock" => create_rosenbrock_function(),
        _ => panic!("unknown objective name: `{}`", name),
    }
}
