#[cfg(test)]
#[path = "../../../tests/unit/format/problem/reader_test.rs"]
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
use crate::extensions::{get_route_modifier, OnlyVehicleActivityCost};
use crate::format::coord_index::CoordIndex;
use crate::format::problem::{deserialize_matrix, deserialize_problem, Matrix};
use crate::format::*;
use crate::utils::get_approx_transportation;
use crate::validation::ValidationContext;
use crate::{get_unique_locations, parse_time};
use hashbrown::HashSet;
use std::cmp::Ordering::Equal;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::models::common::{MultiDimLoad, SingleDimLoad, TimeWindow, ValueDimension};
use vrp_core::models::problem::{ActivityCost, Fleet, TransportCost};
use vrp_core::models::{Extras, Lock, Problem};
use vrp_core::utils::{compare_floats, DefaultRandom, Random};

pub type ApiProblem = crate::format::problem::Problem;

/// Reads specific problem definition from various sources.
pub trait PragmaticProblem {
    /// Reads problem defined in pragmatic format.
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>>;
}

impl<R: Read> PragmaticProblem for (BufReader<R>, Vec<BufReader<R>>) {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        let problem = deserialize_problem(self.0)?;

        let mut matrices = vec![];
        for matrix in self.1 {
            matrices.push(deserialize_matrix(matrix)?);
        }

        map_to_problem_with_matrices(problem, matrices)
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

        map_to_problem_with_matrices(problem, matrices)
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
        map_to_problem_with_matrices(self.0, self.1)
    }
}

impl PragmaticProblem for ApiProblem {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        map_to_problem_with_approx(self)
    }
}

impl PragmaticProblem for (ApiProblem, Option<Vec<Matrix>>) {
    fn read_pragmatic(self) -> Result<Problem, Vec<FormatError>> {
        if let Some(matrices) = self.1 {
            (self.0, matrices).read_pragmatic()
        } else {
            self.0.read_pragmatic()
        }
    }
}

pub struct ProblemProperties {
    has_multi_dimen_capacity: bool,
    has_breaks: bool,
    has_skills: bool,
    has_unreachable_locations: bool,
    has_depots: bool,
    has_reloads: bool,
    has_priorities: bool,
    has_area_limits: bool,
}

fn create_approx_matrices(problem: &ApiProblem) -> Vec<Matrix> {
    const DEFAULT_SPEED: f64 = 10.;
    // get each speed value once
    let speeds = problem
        .fleet
        .profiles
        .iter()
        .map(|profile| profile.speed.unwrap_or(DEFAULT_SPEED))
        .map(|speed| speed.to_bits())
        .collect::<HashSet<u64>>();
    let speeds = speeds.into_iter().map(f64::from_bits).collect::<Vec<_>>();

    let locations = get_unique_locations(&problem);
    let approx_data = get_approx_transportation(&locations, speeds.as_slice());

    problem
        .fleet
        .profiles
        .iter()
        .map(move |profile| {
            let speed = profile.speed.clone().unwrap_or(DEFAULT_SPEED);
            let idx =
                speeds.iter().position(|s| compare_floats(*s, speed) == Equal).expect("Cannot find profile speed");

            Matrix {
                profile: Some(profile.name.clone()),
                timestamp: None,
                travel_times: approx_data[idx].0.clone(),
                distances: approx_data[idx].1.clone(),
                error_codes: None,
            }
        })
        .collect()
}

fn map_to_problem_with_approx(problem: ApiProblem) -> Result<Problem, Vec<FormatError>> {
    let coord_index = CoordIndex::new(&problem);
    let matrices = if coord_index.get_used_types().1 { vec![] } else { create_approx_matrices(&problem) };
    map_to_problem(problem, matrices, coord_index)
}

fn map_to_problem_with_matrices(problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, Vec<FormatError>> {
    let coord_index = CoordIndex::new(&problem);
    map_to_problem(problem, matrices, coord_index)
}

fn map_to_problem(
    api_problem: ApiProblem,
    matrices: Vec<Matrix>,
    coord_index: CoordIndex,
) -> Result<Problem, Vec<FormatError>> {
    ValidationContext::new(&api_problem, Some(&matrices)).validate()?;

    let problem_props = get_problem_properties(&api_problem, &matrices);

    let coord_index = Arc::new(coord_index);
    let transport = create_transport_costs(&api_problem, &matrices).map_err(|err| {
        vec![FormatError::new(
            "E0002".to_string(),
            "cannot create transport costs".to_string(),
            format!("Check matrix routing data: '{}'", err),
        )]
    })?;
    let activity = Arc::new(OnlyVehicleActivityCost::default());
    let fleet = read_fleet(&api_problem, &problem_props, &coord_index);

    // TODO pass random from outside as there might be need to have it initialized with seed
    //      at the moment, this random instance is used only by multi job permutation generator
    let random: Arc<dyn Random + Send + Sync> = Arc::new(DefaultRandom::default());
    let mut job_index = Default::default();
    let (jobs, locks) = read_jobs_with_extra_locks(
        &api_problem,
        &problem_props,
        &coord_index,
        &fleet,
        &transport,
        &mut job_index,
        &random,
    );
    let locks = locks.into_iter().chain(read_locks(&api_problem, &job_index).into_iter()).collect::<Vec<_>>();
    let limits = read_limits(&api_problem).unwrap_or_else(|| Arc::new(|_| (None, None)));
    let mut constraint = create_constraint_pipeline(
        coord_index.clone(),
        &fleet,
        activity.clone(),
        transport.clone(),
        &problem_props,
        &locks,
        limits,
    );

    let objective = create_objective(&api_problem, &mut constraint, &problem_props);
    let constraint = Arc::new(constraint);
    let extras = Arc::new(create_extras(constraint.clone(), &problem_props, job_index, coord_index));

    Ok(Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks,
        constraint,
        activity,
        transport,
        objective,
        extras,
    })
}

fn create_constraint_pipeline(
    coord_index: Arc<CoordIndex>,
    fleet: &Fleet,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    props: &ProblemProperties,
    locks: &[Arc<Lock>],
    limits: TravelLimitFunc,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Box::new(TransportConstraintModule::new(
        activity.clone(),
        transport.clone(),
        limits,
        TIME_CONSTRAINT_CODE,
        DISTANCE_LIMIT_CONSTRAINT_CODE,
        DURATION_LIMIT_CONSTRAINT_CODE,
    )));

    add_capacity_module(&mut constraint, &props);

    if props.has_breaks {
        constraint.add_module(Box::new(BreakModule::new(BREAK_CONSTRAINT_CODE)));
    }

    if props.has_skills {
        constraint.add_module(Box::new(SkillsModule::new(SKILLS_CONSTRAINT_CODE)));
    }

    if props.has_depots {
        constraint.add_module(Box::new(DepotModule::new(DEPOT_CONSTRAINT_CODE)));
    }

    if props.has_priorities {
        constraint.add_module(Box::new(PriorityModule::new(PRIORITY_CONSTRAINT_CODE)));
    }

    if !locks.is_empty() {
        constraint.add_module(Box::new(StrictLockingModule::new(fleet, locks, LOCKING_CONSTRAINT_CODE)));
    }

    if props.has_unreachable_locations {
        constraint.add_module(Box::new(ReachableModule::new(transport.clone(), REACHABLE_CONSTRAINT_CODE)));
    }

    if props.has_area_limits {
        add_area_module(&mut constraint, coord_index);
    }

    constraint
}

fn add_capacity_module(constraint: &mut ConstraintPipeline, props: &ProblemProperties) {
    constraint.add_module(if props.has_reloads {
        let threshold = 0.9;
        if props.has_multi_dimen_capacity {
            Box::new(CapacityConstraintModule::<MultiDimLoad>::new_with_multi_trip(
                CAPACITY_CONSTRAINT_CODE,
                Arc::new(ReloadMultiTrip::new(Box::new(move |capacity| *capacity * threshold))),
            ))
        } else {
            Box::new(CapacityConstraintModule::<SingleDimLoad>::new_with_multi_trip(
                CAPACITY_CONSTRAINT_CODE,
                Arc::new(ReloadMultiTrip::new(Box::new(move |capacity| *capacity * threshold))),
            ))
        }
    } else if props.has_multi_dimen_capacity {
        Box::new(CapacityConstraintModule::<MultiDimLoad>::new(CAPACITY_CONSTRAINT_CODE))
    } else {
        Box::new(CapacityConstraintModule::<SingleDimLoad>::new(CAPACITY_CONSTRAINT_CODE))
    });
}

fn add_area_module(constraint: &mut ConstraintPipeline, coord_index: Arc<CoordIndex>) {
    constraint.add_module(Box::new(AreaModule::new(
        Arc::new(|actor| actor.vehicle.dimens.get_value::<Vec<Area>>("areas")),
        Arc::new(move |location| {
            coord_index
                .get_by_idx(location)
                .map_or_else(|| panic!("cannot find location!"), |location| location.to_lat_lng())
        }),
        AREA_CONSTRAINT_CODE,
    )));
}

fn create_extras(
    constraint: Arc<ConstraintPipeline>,
    props: &ProblemProperties,
    job_index: JobIndex,
    coord_index: Arc<CoordIndex>,
) -> Extras {
    let mut extras = Extras::default();
    extras.insert(
        "capacity_type".to_string(),
        Arc::new((if props.has_multi_dimen_capacity { "multi" } else { "single" }).to_string()),
    );
    extras.insert("coord_index".to_owned(), coord_index);
    extras.insert("job_index".to_owned(), Arc::new(job_index.clone()));

    if props.has_depots {
        extras.insert("route_modifier".to_owned(), Arc::new(get_route_modifier(constraint, job_index)));
    }

    extras
}

fn parse_time_window(tw: &[String]) -> TimeWindow {
    assert_eq!(tw.len(), 2);
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}

fn get_problem_properties(api_problem: &ApiProblem, matrices: &[Matrix]) -> ProblemProperties {
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
        .any(|shift| shift.breaks.as_ref().map_or(false, |b| !b.is_empty()));

    let has_skills = api_problem.plan.jobs.iter().any(|job| job.skills.is_some());
    let has_depots = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|t| t.shifts.iter().any(|s| s.depots.as_ref().map_or(false, |depots| !depots.is_empty())));
    let has_reloads = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|t| t.shifts.iter().any(|s| s.reloads.as_ref().map_or(false, |reloads| !reloads.is_empty())));

    let has_priorities = api_problem.plan.jobs.iter().filter_map(|job| job.priority).any(|priority| priority > 1);
    let has_area_limits = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|v| v.limits.as_ref().and_then(|l| l.allowed_areas.as_ref()).map_or(false, |a| !a.is_empty()));

    ProblemProperties {
        has_multi_dimen_capacity,
        has_breaks,
        has_skills,
        has_unreachable_locations,
        has_depots,
        has_reloads,
        has_priorities,
        has_area_limits,
    }
}
