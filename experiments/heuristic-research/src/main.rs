use heuristic_research::{draw_function_plots, solve_function, solve_vrp, Axes};
use plotters::prelude::*;
use rosomaxa::utils::Environment;

fn main() {
    // TODO make this more configurable
    let vrp_file_path = std::env::args().nth(1);

    let generations = 2000;
    let selection_size = 8;
    let population_type = "rosomaxa";
    let logger = Environment::default().logger;

    let (axes, function_name) = if let Some(vrp_file_path) = vrp_file_path {
        let function_name = "vrp";
        let problem = std::fs::read_to_string(vrp_file_path).expect("cannot read a test file");
        solve_vrp("tsplib", problem, population_type, selection_size, generations, logger);

        (Axes { x: (0.0..2.0, 0.15), y: (0.0..800.), z: (0.0..2.0, 0.15) }, function_name)
    } else {
        let function_name = "rosenbrock";
        let x = -2.;
        let z = -2.;

        solve_function(function_name, population_type, selection_size, vec![x, z], generations, logger);
        (Axes { x: (-2.0..2.0, 0.15), y: (0.0..3610.), z: (-2.0..2.0, 0.15) }, function_name)
    };

    let area = BitMapBackend::new("plot.png", (1024, 768)).into_drawing_area();
    let generation = 100;
    let pitch = 0.;
    let yaw = 0.;

    draw_function_plots(area, generation, pitch, yaw, axes, function_name).unwrap();
}
