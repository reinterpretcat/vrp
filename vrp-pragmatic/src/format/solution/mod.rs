//! Specifies logic to create a "pragmatic" solution and write it into json format.

pub(crate) mod activity_matcher;

mod break_writer;
use self::break_writer::insert_reserved_times_as_breaks;

mod extensions;

mod geo_serializer;
pub use self::geo_serializer::*;

mod initial_reader;
pub use self::initial_reader::read_init_solution;

mod model;
pub use self::model::*;

mod solution_writer;
pub(crate) use self::solution_writer::create_solution;

use super::*;
use crate::{format_time, parse_time};
use std::io::{BufWriter, Write};
use vrp_core::prelude::GenericError;

type ApiActivity = model::Activity;
type ApiSolution = model::Solution;
type ApiSchedule = model::Schedule;
type ApiMetrics = model::Metrics;
type ApiGeneration = model::Generation;
type AppPopulation = model::Population;
type ApiIndividual = model::Individual;
type DomainProblem = vrp_core::models::Problem;
type DomainSolution = vrp_core::models::Solution;
type DomainSchedule = vrp_core::models::common::Schedule;
type DomainLocation = vrp_core::models::common::Location;
type DomainExtras = vrp_core::models::Extras;

/// Specifies possible options for solution output.
#[derive(Default)]
pub enum PragmaticOutputType {
    /// Only pragmatic is needed.
    #[default]
    OnlyPragmatic,
    /// Only geojson is needed.
    OnlyGeoJson,
    /// Pragmatic and geojson is returned. Geojson features are embedded inside extras property.
    Combined,
}

/// Writes solution in pragmatic format variation defined by output type argument.
pub fn write_pragmatic<W: Write>(
    problem: &DomainProblem,
    solution: &DomainSolution,
    output_type: PragmaticOutputType,
    writer: &mut BufWriter<W>,
) -> Result<(), GenericError> {
    let solution = create_solution(problem, solution, &output_type);

    match output_type {
        PragmaticOutputType::OnlyPragmatic | PragmaticOutputType::Combined => {
            serialize_solution(&solution, writer).map_err(|err| err.to_string())?;
        }
        PragmaticOutputType::OnlyGeoJson => {
            serialize_solution_as_geojson(problem, &solution, writer).map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

fn map_code_reason(code: ViolationCode) -> (&'static str, &'static str) {
    match code {
        SKILL_CONSTRAINT_CODE => ("SKILL_CONSTRAINT", "cannot serve required skill"),
        TIME_CONSTRAINT_CODE => ("TIME_WINDOW_CONSTRAINT", "cannot be visited within time window"),
        CAPACITY_CONSTRAINT_CODE => ("CAPACITY_CONSTRAINT", "does not fit into any vehicle due to capacity"),
        REACHABLE_CONSTRAINT_CODE => ("REACHABLE_CONSTRAINT", "location unreachable"),
        DISTANCE_LIMIT_CONSTRAINT_CODE => {
            ("MAX_DISTANCE_CONSTRAINT", "cannot be assigned due to max distance constraint of vehicle")
        }
        DURATION_LIMIT_CONSTRAINT_CODE => {
            ("MAX_DURATION_CONSTRAINT", "cannot be assigned due to max duration constraint of vehicle")
        }
        BREAK_CONSTRAINT_CODE => ("BREAK_CONSTRAINT", "break is not assignable"),
        LOCKING_CONSTRAINT_CODE => ("LOCKING_CONSTRAINT", "cannot be served due to relation lock"),
        AREA_CONSTRAINT_CODE => ("AREA_CONSTRAINT", "cannot be assigned due to area constraint"),
        TOUR_SIZE_CONSTRAINT_CODE => {
            ("TOUR_SIZE_CONSTRAINT", "cannot be assigned due to tour size constraint of vehicle")
        }
        TOUR_ORDER_CONSTRAINT_CODE => ("TOUR_ORDER_CONSTRAINT", "cannot be assigned due to tour order constraint"),
        GROUP_CONSTRAINT_CODE => ("GROUP_CONSTRAINT", "cannot be assigned due to group constraint"),
        COMPATIBILITY_CONSTRAINT_CODE => {
            ("COMPATIBILITY_CONSTRAINT", "cannot be assigned due to compatibility constraint")
        }
        RELOAD_RESOURCE_CONSTRAINT_CODE => {
            ("RELOAD_RESOURCE_CONSTRAINT", "cannot be assigned due to reload resource constraint")
        }
        RECHARGE_CONSTRAINT_CODE => ("RECHARGE_CONSTRAINT_CODE", "cannot be assigned due to recharge constraint"),
        MIN_VEHICLE_SHIFTS_CONSTRAINT_CODE => {
            ("MIN_SHIFT_CONSTRAINT", "cannot be assigned due to minimum shift requirement")
        }
        _ => ("NO_REASON_FOUND", "unknown"),
    }
}

fn map_reason_code(reason: &str) -> ViolationCode {
    match reason {
        "SKILL_CONSTRAINT" => SKILL_CONSTRAINT_CODE,
        "TIME_WINDOW_CONSTRAINT" => TIME_CONSTRAINT_CODE,
        "CAPACITY_CONSTRAINT" => CAPACITY_CONSTRAINT_CODE,
        "REACHABLE_CONSTRAINT" => REACHABLE_CONSTRAINT_CODE,
        "MAX_DISTANCE_CONSTRAINT" => DISTANCE_LIMIT_CONSTRAINT_CODE,
        "MAX_DURATION_CONSTRAINT" => DURATION_LIMIT_CONSTRAINT_CODE,
        "BREAK_CONSTRAINT" => BREAK_CONSTRAINT_CODE,
        "LOCKING_CONSTRAINT" => LOCKING_CONSTRAINT_CODE,
        "AREA_CONSTRAINT" => AREA_CONSTRAINT_CODE,
        "TOUR_SIZE_CONSTRAINT" => TOUR_SIZE_CONSTRAINT_CODE,
        "TOUR_ORDER_CONSTRAINT" => TOUR_ORDER_CONSTRAINT_CODE,
        "GROUP_CONSTRAINT" => GROUP_CONSTRAINT_CODE,
        "COMPATIBILITY_CONSTRAINT" => COMPATIBILITY_CONSTRAINT_CODE,
        "RELOAD_RESOURCE_CONSTRAINT" => RELOAD_RESOURCE_CONSTRAINT_CODE,
        "RECHARGE_CONSTRAINT_CODE" => RECHARGE_CONSTRAINT_CODE,
        "MIN_SHIFT_CONSTRAINT" => MIN_VEHICLE_SHIFTS_CONSTRAINT_CODE,
        _ => ViolationCode::unknown(),
    }
}
