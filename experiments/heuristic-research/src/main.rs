use heuristic_research::run_solver;
use rosomaxa::utils::Environment;

fn main() {
    let x = -2.;
    let z = -2.;
    let generations = 2000;
    let selection_size = 8;
    let function_name = "rosenbrock";
    let population_type = "default";
    let logger = Environment::default().logger;

    run_solver(function_name, population_type, selection_size, vec![x, z], generations, logger);
}
