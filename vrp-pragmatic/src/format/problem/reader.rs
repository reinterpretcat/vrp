#[cfg(test)]
#[path = "../../../tests/unit/format/problem/reader_test.rs"]
mod reader_test;

#[path = "./job_reader.rs"]
mod job_reader;

#[path = "./fleet_reader.rs"]
mod fleet_reader;

#[path = "./objective_reader.rs"]
mod objective_reader;

#[path = "./clustering_reader.rs"]
mod clustering_reader;

use self::clustering_reader::create_cluster_config;
use self::fleet_reader::{create_transport_costs, read_fleet, read_travel_limits};
use self::job_reader::{read_jobs_with_extra_locks, read_locks};
use self::objective_reader::create_objective;
use crate::constraints::*;
use crate::extensions::{get_route_modifier, OnlyVehicleActivityCost};
use crate::format::coord_index::CoordIndex;
use crate::format::problem::*;
use crate::format::*;
use crate::utils::get_approx_transportation;
use crate::validation::ValidationContext;
use crate::{get_unique_locations, parse_time};
use hashbrown::HashSet;
use std::cmp::Ordering::Equal;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::{Extras, Lock, Problem};
use vrp_core::prelude::*;
use vrp_core::rosomaxa::utils::CollectGroupBy;
use vrp_core::solver::processing::VicinityDimension;

pub type ApiProblem = crate::format::problem::Problem;
pub type CoreFleet = vrp_core::models::problem::Fleet;

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
    has_dispatch: bool,
    has_reloads: bool,
    has_order: bool,
    has_group: bool,
    has_compatibility: bool,
    has_tour_size_limits: bool,
    max_job_value: Option<f64>,
    max_area_value: Option<f64>,
}

/// Creates a matrices using approximation.
pub fn create_approx_matrices(problem: &ApiProblem) -> Vec<Matrix> {
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

    let locations = get_unique_locations(problem);
    let approx_data = get_approx_transportation(&locations, speeds.as_slice());

    problem
        .fleet
        .profiles
        .iter()
        .map(move |profile| {
            let speed = profile.speed.unwrap_or(DEFAULT_SPEED);
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
    ValidationContext::new(&api_problem, Some(&matrices), &coord_index).validate()?;

    let problem_props = get_problem_properties(&api_problem, &matrices);

    let coord_index = Arc::new(coord_index);
    let fleet = read_fleet(&api_problem, &problem_props, &coord_index);
    let reserved_times_index = read_reserved_times_index(&api_problem, &fleet);

    let transport = create_transport_costs(&api_problem, &matrices).map_err(|err| {
        vec![FormatError::new(
            "E0002".to_string(),
            "cannot create transport costs".to_string(),
            format!("check matrix routing data: '{}'", err),
        )]
    })?;
    let activity: Arc<dyn ActivityCost + Send + Sync> = Arc::new(OnlyVehicleActivityCost::default());

    let (transport, activity) = if reserved_times_index.is_empty() {
        (transport, activity)
    } else {
        DynamicTransportCost::new(reserved_times_index.clone(), transport)
            .and_then(|transport| {
                DynamicActivityCost::new(reserved_times_index.clone()).map(|activity| (transport, activity))
            })
            .map_err(|err| {
                vec![FormatError::new(
                    "E0002".to_string(),
                    "cannot create transport costs".to_string(),
                    format!("check fleet definition: '{}'", err),
                )]
            })
            .map::<(Arc<dyn TransportCost + Send + Sync>, Arc<dyn ActivityCost + Send + Sync>), _>(
                |(transport, activity)| (Arc::new(transport), Arc::new(activity)),
            )?
    };

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
    let limits = read_travel_limits(&api_problem).unwrap_or_else(|| Arc::new(|_| (None, None)));
    let mut constraint =
        create_constraint_pipeline(&jobs, &fleet, transport.clone(), activity.clone(), &problem_props, &locks, limits);

    let objective = create_objective(&api_problem, &mut constraint, &problem_props);
    let constraint = Arc::new(constraint);
    let extras = Arc::new(
        create_extras(&api_problem, constraint.clone(), &problem_props, job_index, coord_index, reserved_times_index)
            .map_err(|err| {
            // TODO make sure that error matches actual reason
            vec![FormatError::new(
                "E0002".to_string(),
                "cannot create transport costs".to_string(),
                format!("check clustering config: '{}'", err),
            )]
        })?,
    );

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

fn read_reserved_times_index(api_problem: &ApiProblem, fleet: &CoreFleet) -> ReservedTimesIndex {
    let breaks_map = api_problem
        .fleet
        .vehicles
        .iter()
        .flat_map(|vehicle| {
            vehicle.shifts.iter().enumerate().flat_map(move |(shift_idx, shift)| {
                shift.breaks.iter().flat_map(|br| br.iter()).filter_map(move |br| match br {
                    VehicleBreak::Required { time, duration } => {
                        Some((vehicle.type_id.clone(), shift_idx, time.clone(), *duration))
                    }
                    VehicleBreak::Optional { .. } => None,
                })
            })
        })
        .collect_group_by_key(|(type_id, shift_idx, _, _)| (type_id.clone(), *shift_idx));

    fleet
        .actors
        .iter()
        .filter_map(|actor| {
            let type_id = actor.vehicle.dimens.get_value::<String>("type_id").unwrap().clone();
            let shift_idx = *actor.vehicle.dimens.get_value::<usize>("shift_index").unwrap();

            let times = breaks_map
                .get(&(type_id, shift_idx))
                .iter()
                .flat_map(|data| data.iter())
                .map(|(_, _, time, duration)| match time {
                    VehicleRequiredBreakTime::ExactTime(time) => {
                        let time = parse_time(time);
                        TimeSpan::Window(TimeWindow::new(time, time + duration))
                    }
                    VehicleRequiredBreakTime::OffsetTime(offset) => {
                        TimeSpan::Offset(TimeOffset::new(*offset, *offset + duration))
                    }
                })
                .collect::<Vec<_>>();

            if times.is_empty() {
                None
            } else {
                Some((actor.clone(), times))
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn create_constraint_pipeline(
    jobs: &Jobs,
    fleet: &CoreFleet,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    props: &ProblemProperties,
    locks: &[Arc<Lock>],
    limits: TravelLimitFunc,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();

    if props.has_unreachable_locations {
        constraint.add_module(Arc::new(ReachableModule::new(transport.clone(), REACHABLE_CONSTRAINT_CODE)));
    }

    constraint.add_module(Arc::new(TransportConstraintModule::new(
        transport.clone(),
        activity.clone(),
        limits,
        TIME_CONSTRAINT_CODE,
        DISTANCE_LIMIT_CONSTRAINT_CODE,
        DURATION_LIMIT_CONSTRAINT_CODE,
    )));

    add_capacity_module(&mut constraint, props, activity.clone(), transport.clone());

    if props.has_breaks {
        constraint.add_module(Arc::new(BreakModule::new(activity.clone(), transport.clone(), BREAK_CONSTRAINT_CODE)));
    }

    if props.has_compatibility {
        constraint.add_module(Arc::new(CompatibilityModule::new(COMPATIBILITY_CONSTRAINT_CODE, COMPATIBILITY_KEY)));
    }

    if props.has_group {
        constraint.add_module(Arc::new(GroupModule::new(jobs.size(), GROUP_CONSTRAINT_CODE, GROUP_KEY)));
    }

    if props.has_skills {
        constraint.add_module(Arc::new(SkillsModule::new(SKILL_CONSTRAINT_CODE)));
    }

    if props.has_dispatch {
        constraint.add_module(Arc::new(DispatchModule::new(DISPATCH_CONSTRAINT_CODE)));
    }

    if !locks.is_empty() {
        constraint.add_module(Arc::new(StrictLockingModule::new(fleet, locks, LOCKING_CONSTRAINT_CODE)));
    }

    if props.has_tour_size_limits {
        add_tour_size_module(&mut constraint)
    }

    constraint
}

fn add_capacity_module(
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
) {
    constraint.add_module(if props.has_reloads {
        let threshold = 0.9;
        if props.has_multi_dimen_capacity {
            Arc::new(CapacityConstraintModule::<MultiDimLoad>::new_with_multi_trip(
                CAPACITY_CONSTRAINT_CODE,
                Arc::new(ReloadMultiTrip::new(activity, transport, Box::new(move |capacity| *capacity * threshold))),
            ))
        } else {
            Arc::new(CapacityConstraintModule::<SingleDimLoad>::new_with_multi_trip(
                CAPACITY_CONSTRAINT_CODE,
                Arc::new(ReloadMultiTrip::new(activity, transport, Box::new(move |capacity| *capacity * threshold))),
            ))
        }
    } else if props.has_multi_dimen_capacity {
        Arc::new(CapacityConstraintModule::<MultiDimLoad>::new(CAPACITY_CONSTRAINT_CODE))
    } else {
        Arc::new(CapacityConstraintModule::<SingleDimLoad>::new(CAPACITY_CONSTRAINT_CODE))
    });
}

fn add_tour_size_module(constraint: &mut ConstraintPipeline) {
    constraint.add_module(Arc::new(TourSizeModule::new(
        Arc::new(|actor| actor.vehicle.dimens.get_value::<usize>("tour_size").cloned()),
        TOUR_SIZE_CONSTRAINT_CODE,
    )));
}

fn create_extras(
    api_problem: &ApiProblem,
    constraint: Arc<ConstraintPipeline>,
    props: &ProblemProperties,
    job_index: JobIndex,
    coord_index: Arc<CoordIndex>,
    reserved_times_index: ReservedTimesIndex,
) -> Result<Extras, String> {
    let mut extras = Extras::default();

    extras.insert("coord_index".to_owned(), coord_index);
    extras.insert("job_index".to_owned(), Arc::new(job_index.clone()));
    extras.insert("reserved_times_index".to_owned(), Arc::new(reserved_times_index));

    if props.has_dispatch {
        extras.insert("route_modifier".to_owned(), Arc::new(get_route_modifier(constraint, job_index)));
    }

    if let Some(config) = create_cluster_config(api_problem)? {
        extras.set_cluster_config(config);
    }

    Ok(extras)
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
    let max_job_value = api_problem
        .plan
        .jobs
        .iter()
        .filter_map(|job| job.value)
        .filter(|value| *value > 0.)
        .max_by(|a, b| compare_floats(*a, *b));

    let max_area_value = api_problem
        .fleet
        .vehicles
        .iter()
        .flat_map(|vehicle| vehicle.limits.iter())
        .flat_map(|limits| limits.areas.iter())
        .flat_map(|areas| areas.iter())
        .flat_map(|areas| areas.iter())
        .filter_map(|limit| if limit.job_value > 0. { Some(limit.job_value) } else { None })
        .max_by(|a, b| compare_floats(*a, *b));

    let has_dispatch = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|t| t.shifts.iter().any(|s| s.dispatch.as_ref().map_or(false, |dispatch| !dispatch.is_empty())));
    let has_reloads = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|t| t.shifts.iter().any(|s| s.reloads.as_ref().map_or(false, |reloads| !reloads.is_empty())));

    let has_order = api_problem
        .plan
        .jobs
        .iter()
        .flat_map(get_job_tasks)
        .filter_map(|job_task| job_task.order)
        .any(|order| order > 0);

    let has_group = api_problem.plan.jobs.iter().any(|job| job.group.is_some());
    let has_compatibility = api_problem.plan.jobs.iter().any(|job| job.compatibility.is_some());
    let has_tour_size_limits =
        api_problem.fleet.vehicles.iter().any(|v| v.limits.as_ref().map_or(false, |l| l.tour_size.is_some()));

    ProblemProperties {
        has_multi_dimen_capacity,
        has_breaks,
        has_skills,
        has_unreachable_locations,
        has_dispatch,
        has_reloads,
        has_order,
        has_group,
        has_compatibility,
        has_tour_size_limits,
        max_job_value,
        max_area_value,
    }
}
