#[cfg(test)]
#[path = "../../../tests/unit/solver/vrp/vrp_test.rs"]
mod vrp_test;

mod population;
pub use self::population::{get_population_desc, get_population_fitness_fn};

use super::*;
use std::io::BufWriter;
use vrp_scientific::core::models::common::Footprint;
use vrp_scientific::core::prelude::*;
use vrp_scientific::core::solver::RefinementContext;
use vrp_scientific::lilim::{LilimProblem, LilimSolution};
use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};
use vrp_scientific::tsplib::{TsplibProblem, TsplibSolution};

/// Solves VRP of the given format type.
pub fn solve_vrp(
    format_type: &str,
    problem: String,
    population_type: &str,
    selection_size: usize,
    generations: usize,
    logger: InfoLogger,
) {
    let is_rounded = true;
    let is_experimental = true;
    let logger = create_info_logger_proxy(logger);

    let problem = match format_type {
        "tsplib" => problem.read_tsplib(is_rounded),
        "solomon" => problem.read_solomon(is_rounded),
        "lilim" => problem.read_lilim(is_rounded),
        _ => panic!("unknown format: {format_type}"),
    }
    .unwrap();

    let problem = Arc::new(problem);

    let environment = Arc::new(Environment {
        logger: logger.clone(),
        is_experimental,
        ..Environment::new_with_time_quota(Some(300))
    });
    let footprint = Footprint::new(problem.as_ref());
    let population =
        get_population(footprint, population_type, problem.goal.clone(), environment.clone(), selection_size);
    let telemetry_mode = TelemetryMode::OnlyLogging { logger: logger.clone(), log_best: 100, log_population: 1000 };

    let config = VrpConfigBuilder::new(problem.clone())
        .set_environment(environment.clone())
        .set_telemetry_mode(telemetry_mode.clone())
        .prebuild()
        .expect("cannot prebuild vrp configuration")
        .with_max_generations(Some(generations))
        .with_context(RefinementContext::new(problem.clone(), population, telemetry_mode, environment))
        .build()
        .expect("cannot build config");

    let solution = Solver::new(problem, config).solve().expect("cannot solve problem");

    let mut writer = BufWriter::new(Vec::new());
    match format_type {
        "tsplib" => solution.write_tsplib(&mut writer),
        "solomon" => solution.write_solomon(&mut writer),
        "lilim" => solution.write_lilim(&mut writer),
        _ => unreachable!("unknown format: {}", format_type),
    }
    .expect("cannot write solution");

    let result = String::from_utf8(writer.into_inner().expect("cannot use writer")).expect("cannot create string");

    (logger)(&result);
}
