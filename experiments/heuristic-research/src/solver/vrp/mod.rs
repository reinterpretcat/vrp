#[cfg(test)]
#[path = "../../../tests/unit/solver/vrp/vrp_test.rs"]
mod vrp_test;

extern crate serde_json;
use serde::{Deserialize, Serialize};

use super::*;
use std::io::BufWriter;
use vrp_scientific::common::RoutingMode;
use vrp_scientific::core::prelude::*;
use vrp_scientific::core::solver::RefinementContext;
use vrp_scientific::lilim::{LilimProblem, LilimSolution};
use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};
use vrp_scientific::tsplib::{TsplibProblem, TsplibSolution};

mod conversion;

/// Represents VRP solution as directed graph.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct DataGraph {
    /// Nodes data: x and y coordinate.
    pub nodes: Vec<GraphNode>,
    /// Edges data: source and target nodes.
    pub edges: Vec<GraphEdge>,
}

/// Node of a graph.
#[derive(Clone, Deserialize, Serialize)]
pub struct GraphNode {
    pub x: Float,
    pub y: Float,
}

/// Edge of a graph.
#[derive(Clone, Deserialize, Serialize)]
pub struct GraphEdge {
    pub source: usize,
    pub target: usize,
}

/// Solves VRP of the given format type.
pub fn solve_vrp(
    format_type: &str,
    problem: String,
    population_type: &str,
    selection_size: usize,
    generations: usize,
    logger: InfoLogger,
) {
    let is_experimental = true;
    let logger = create_info_logger_proxy(logger);

    let problem = match format_type {
        "tsplib" => problem.read_tsplib(RoutingMode::ScaleWithRound(1000.)),
        "solomon" => problem.read_solomon(RoutingMode::ScaleWithRound(1000.)),
        "lilim" => problem.read_lilim(RoutingMode::ScaleWithRound(1000.)),
        _ => panic!("unknown format: {format_type}"),
    }
    .unwrap();

    let problem = Arc::new(problem);

    let environment = Arc::new(Environment {
        logger: logger.clone(),
        is_experimental,
        ..Environment::new_with_time_quota(Some(300))
    });
    let population = get_population(population_type, problem.goal.clone(), environment.clone(), selection_size);
    let telemetry_mode = TelemetryMode::OnlyLogging {
        logger: logger.clone(),
        log_best: 100,
        log_population: 1000,
        dump_population: false,
    };

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
