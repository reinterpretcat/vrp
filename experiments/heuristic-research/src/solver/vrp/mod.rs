#[cfg(test)]
#[path = "../../../tests/unit/solver/vrp_test.rs"]
mod vrp_test;

extern crate serde_json;
use serde::Serialize;

use super::*;
use std::io::BufWriter;
use std::ops::Deref;
use vrp_scientific::core::prelude::*;
use vrp_scientific::core::solver::RefinementContext;
use vrp_scientific::lilim::{LilimProblem, LilimSolution};
use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};
use vrp_scientific::tsplib::{TsplibProblem, TsplibSolution};

mod conversion;

#[derive(Clone, Serialize)]
pub struct DataGraph {
    /// Nodes data: x and y coordinate.
    pub nodes: Vec<GraphNode>,
    /// Edges data: source and target nodes.
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Serialize)]
pub struct GraphNode {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Serialize)]
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
    let is_rounded = true;

    let problem = match format_type {
        "tsplib" => problem.read_tsplib(is_rounded),
        "solomon" => problem.read_solomon(is_rounded),
        "lilim" => problem.read_lilim(is_rounded),
        _ => panic!("unknown format: {}", format_type),
    }
    .unwrap();

    let problem = Arc::new(problem);

    let environment = Arc::new(Environment { logger: logger.clone(), ..Environment::new_with_time_quota(Some(10)) });
    let population = get_population(population_type, problem.objective.clone(), environment.clone(), selection_size);
    let telemetry_mode = TelemetryMode::OnlyLogging {
        logger: logger.clone(),
        log_best: 100,
        log_population: 1000,
        dump_population: false,
    };

    let config = create_default_config_builder(problem.clone(), environment.clone(), telemetry_mode.clone())
        .with_max_generations(Some(generations))
        .with_context(RefinementContext::new(problem.clone(), population, telemetry_mode, environment))
        .build()
        .expect("cannot build config");

    let (solution, cost, _) = Solver::new(problem, config).solve().expect("cannot solve problem");

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    match format_type {
        "tsplib" => (&solution, cost).write_tsplib(writer),
        "solomon" => (&solution, cost).write_solomon(writer),
        "lilim" => (&solution, cost).write_lilim(writer),
        _ => unreachable!("unknown format: {}", format_type),
    }
    .expect("cannot write solution");

    logger.deref()(&buffer);
}
