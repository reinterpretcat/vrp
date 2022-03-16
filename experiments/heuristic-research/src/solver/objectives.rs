use rosomaxa::example::{create_rosenbrock_function, VectorFunction};

/// Returns objective function by its name.
pub fn get_objective_function_by_name(name: &str) -> VectorFunction {
    match name {
        "rosenbrock" => create_rosenbrock_function(),
        _ => panic!("unknown objective name: `{}`", name),
    }
}
