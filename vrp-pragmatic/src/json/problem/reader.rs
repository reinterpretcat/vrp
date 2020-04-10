#[cfg(test)]
#[path = "../../../tests/unit/json/problem/reader_test.rs"]
mod reader_test;

#[path = "./job_reader.rs"]
mod job_reader;

#[path = "./fleet_reader.rs"]
mod fleet_reader;

#[path = "./objective_reader.rs"]
mod objective_reader;

use self::fleet_reader::{create_transport_costs, read_fleet, read_limits};
use self::job_reader::{read_jobs_with_extra_locks, read_locks};
use self::objective_reader::create_objective;
use crate::constraints::*;
use crate::extensions::{MultiDimensionalCapacity, OnlyVehicleActivityCost};
use crate::json::coord_index::CoordIndex;
use crate::json::problem::{deserialize_matrix, deserialize_problem, Matrix};
use crate::json::*;
use crate::utils::get_approx_transportation;
use crate::validation::ValidationContext;
use crate::{get_unique_locations, parse_time};
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::iter::FromIterator;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::models::common::{Dimensions, TimeWindow, ValueDimension};
use vrp_core::models::problem::{ActivityCost, Fleet, Job, TransportCost};
use vrp_core::models::{Extras, Lock, Problem};

pub type ApiProblem = crate::json::problem::Problem;
pub type JobIndex = HashMap<String, Job>;

/// Reads specific problem definition from various sources.
pub trait PragmaticProblem {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>>;
}

impl<R: Read> PragmaticProblem for (BufReader<R>, Vec<BufReader<R>>) {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        let problem = deserialize_problem(self.0)?;

        let mut matrices = vec![];
        for matrix in self.1 {
            matrices.push(deserialize_matrix(matrix)?);
        }

        map_to_problem(problem, matrices)
    }
}

impl<R: Read> PragmaticProblem for BufReader<R> {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        let problem = deserialize_problem(self)?;

        map_to_problem_with_approx(problem)
    }
}

impl PragmaticProblem for (String, Vec<String>) {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        let problem = deserialize_problem(BufReader::new(self.0.as_bytes()))?;

        let mut matrices = vec![];
        for matrix in self.1 {
            matrices.push(deserialize_matrix(BufReader::new(matrix.as_bytes()))?);
        }

        map_to_problem(problem, matrices)
    }
}

impl PragmaticProblem for String {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        let problem = deserialize_problem(BufReader::new(self.as_bytes()))?;

        map_to_problem_with_approx(problem)
    }
}

impl PragmaticProblem for (ApiProblem, Vec<Matrix>) {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        map_to_problem(self.0, self.1)
    }
}

impl PragmaticProblem for ApiProblem {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        map_to_problem_with_approx(self)
    }
}

pub struct ProblemProperties {
    has_multi_dimen_capacity: bool,
    has_breaks: bool,
    has_skills: bool,
    has_unreachable_locations: bool,
    has_reload: bool,
    has_priorities: bool,
}

/// A format error.
#[derive(Clone, Debug)]
pub struct FormatError {
    /// An error code in registry.
    pub code: String,
    /// A possible error cause.
    pub cause: String,
    /// An action to take in order to recover from error.
    pub action: String,
    /// A details about exception.
    pub details: Option<String>,
}

impl FormatError {
    /// Creates a new instance of `FormatError` action without details.
    pub fn new(code: String, cause: String, action: String) -> Self {
        Self { code, cause, action, details: None }
    }

    /// Creates a new instance of `FormatError` action.
    pub fn new_with_details(code: String, cause: String, action: String, details: String) -> Self {
        Self { code, cause, action, details: Some(details) }
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, cause: '{}', action: '{}'.", self.code, self.cause, self.action)
    }
}

fn map_to_problem_with_approx(problem: ApiProblem) -> Result<Problem, Vec<FormatError>> {
    let locations = get_unique_locations(&problem);
    let (durations, distances) = get_approx_transportation(&locations, 10.);

    let durations = durations.into_iter().map(|d| d.round() as i64).collect::<Vec<_>>();
    let distances = distances.into_iter().map(|d| d.round() as i64).collect::<Vec<_>>();

    let matrices = problem
        .fleet
        .profiles
        .iter()
        .map(move |profile| Matrix {
            profile: profile.name.clone(),
            timestamp: None,
            travel_times: durations.clone(),
            distances: distances.clone(),
            error_codes: None,
        })
        .collect();

    map_to_problem(problem, matrices)
}

fn map_to_problem(api_problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, Vec<FormatError>> {
    ValidationContext::new(&api_problem, Some(&matrices)).validate()?;

    let problem_props = get_problem_properties(&api_problem, &matrices);

    let coord_index = CoordIndex::new(&api_problem);
    let transport = create_transport_costs(&api_problem, &matrices).map_err(|err| {
        vec![FormatError::new(
            "E0002".to_string(),
            "cannot create transport costs".to_string(),
            format!("Check matrix routing data: '{}'", err),
        )]
    })?;
    let activity = Arc::new(OnlyVehicleActivityCost::default());
    let fleet = read_fleet(&api_problem, &problem_props, &coord_index);

    let mut job_index = Default::default();
    let (jobs, locks) =
        read_jobs_with_extra_locks(&api_problem, &problem_props, &coord_index, &fleet, &transport, &mut job_index);
    let locks = locks.into_iter().chain(read_locks(&api_problem, &job_index).into_iter()).collect();
    let limits = read_limits(&api_problem).unwrap_or_else(|| Arc::new(|_| (None, None)));
    let extras = Arc::new(create_extras(&problem_props, coord_index));
    let mut constraint =
        create_constraint_pipeline(&fleet, activity.clone(), transport.clone(), &problem_props, &locks, limits);

    let objective = Arc::new(create_objective(&api_problem, &mut constraint, &problem_props));

    Ok(Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks,
        constraint: Arc::new(constraint),
        activity,
        transport,
        objective,
        extras,
    })
}

fn create_constraint_pipeline(
    fleet: &Fleet,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    props: &ProblemProperties,
    locks: &Vec<Arc<Lock>>,
    limits: TravelLimitFunc,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Box::new(TransportConstraintModule::new(
        activity.clone(),
        transport.clone(),
        limits,
        1,
        2,
        3,
    )));

    add_capacity_module(&mut constraint, &props);

    if props.has_breaks {
        constraint.add_module(Box::new(BreakModule::new(BREAK_CONSTRAINT_CODE, Some(-100.), false)));
    }

    if props.has_skills {
        constraint.add_module(Box::new(SkillsModule::new(SKILLS_CONSTRAINT_CODE)));
    }

    if props.has_priorities {
        constraint.add_module(Box::new(PriorityModule::new(PRIORITY_CONSTRAINT_CODE)));
    }

    if !locks.is_empty() {
        constraint.add_module(Box::new(StrictLockingModule::new(fleet, locks.clone(), LOCKING_CONSTRAINT_CODE)));
    }

    if props.has_unreachable_locations {
        constraint.add_module(Box::new(ReachableModule::new(transport.clone(), REACHABLE_CONSTRAINT_CODE)));
    }

    constraint
}

fn add_capacity_module(constraint: &mut ConstraintPipeline, props: &ProblemProperties) {
    constraint.add_module(if props.has_reload {
        let threshold = 0.9;
        if props.has_multi_dimen_capacity {
            Box::new(CapacityConstraintModule::<MultiDimensionalCapacity>::new_with_multi_trip(
                CAPACITY_CONSTRAINT_CODE,
                Arc::new(ReloadMultiTrip::new(Box::new(|capacity| *capacity * 0.9))),
            ))
        } else {
            Box::new(CapacityConstraintModule::<i32>::new_with_multi_trip(
                CAPACITY_CONSTRAINT_CODE,
                Arc::new(ReloadMultiTrip::new(Box::new(move |capacity| (*capacity as f64 * threshold).round() as i32))),
            ))
        }
    } else {
        if props.has_multi_dimen_capacity {
            Box::new(CapacityConstraintModule::<MultiDimensionalCapacity>::new(CAPACITY_CONSTRAINT_CODE))
        } else {
            Box::new(CapacityConstraintModule::<i32>::new(CAPACITY_CONSTRAINT_CODE))
        }
    });
}

fn create_extras(props: &ProblemProperties, coord_index: CoordIndex) -> Extras {
    let mut extras = Extras::default();
    extras.insert(
        "capacity_type".to_string(),
        Box::new((if props.has_multi_dimen_capacity { "multi" } else { "single" }).to_string()),
    );
    extras.insert("coord_index".to_owned(), Box::new(coord_index));

    extras
}

fn parse_time_window(tw: &Vec<String>) -> TimeWindow {
    assert_eq!(tw.len(), 2);
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}

fn get_problem_properties(api_problem: &ApiProblem, matrices: &Vec<Matrix>) -> ProblemProperties {
    let has_unreachable_locations = matrices.iter().any(|m| m.error_codes.is_some());
    let has_multi_dimen_capacity = api_problem.fleet.vehicles.iter().any(|t| t.capacity.len() > 1)
        || api_problem.plan.jobs.iter().any(|job| {
            job.pickups
                .iter()
                .chain(job.deliveries.iter())
                .flat_map(|tasks| tasks.iter())
                .any(|task| task.demand.as_ref().map_or(false, |d| d.len() > 1))
        });
    let has_breaks = api_problem
        .fleet
        .vehicles
        .iter()
        .flat_map(|t| &t.shifts)
        .any(|shift| shift.breaks.as_ref().map_or(false, |b| b.len() > 0));
    let has_skills = api_problem.plan.jobs.iter().any(|job| job.skills.is_some());
    let has_reload = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|t| t.shifts.iter().any(|s| s.reloads.as_ref().map_or(false, |reloads| !reloads.is_empty())));

    let has_priorities = api_problem.plan.jobs.iter().filter_map(|job| job.priority).any(|priority| priority > 1);

    ProblemProperties {
        has_multi_dimen_capacity,
        has_breaks,
        has_skills,
        has_unreachable_locations,
        has_reload,
        has_priorities,
    }
}

fn add_skills(dimens: &mut Dimensions, skills: &Option<Vec<String>>) {
    if let Some(skills) = skills {
        dimens.set_value("skills", HashSet::<String>::from_iter(skills.iter().cloned()));
    }
}
