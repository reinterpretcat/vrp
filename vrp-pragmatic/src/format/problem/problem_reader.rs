use super::*;
use crate::format::problem::clustering_reader::create_cluster_config;
use crate::format::problem::fleet_reader::*;
use crate::format::problem::goal_reader::create_goal_context;
use crate::format::problem::job_reader::{read_jobs_with_extra_locks, read_locks};
use crate::format::{FormatError, JobIndex};
use crate::validation::ValidationContext;
use crate::{parse_time, CoordIndex};
use vrp_core::construction::enablers::*;
use vrp_core::construction::heuristics::StateKeyRegistry;
use vrp_core::models::common::{TimeOffset, TimeSpan, TimeWindow};
use vrp_core::models::ExtrasBuilder;
use vrp_core::solver::processing::{ReservedTimeExtras, VicinityDimensionExtras};

pub(super) fn map_to_problem_with_approx(problem: ApiProblem) -> Result<CoreProblem, MultiFormatError> {
    let coord_index = CoordIndex::new(&problem);
    let matrices = if coord_index.has_indices() { vec![] } else { create_approx_matrices(&problem) };
    map_to_problem(problem, matrices, coord_index)
}

pub(super) fn map_to_problem_with_matrices(
    problem: ApiProblem,
    matrices: Vec<Matrix>,
) -> Result<CoreProblem, MultiFormatError> {
    let coord_index = CoordIndex::new(&problem);
    map_to_problem(problem, matrices, coord_index)
}

pub(super) fn map_to_problem(
    api_problem: ApiProblem,
    matrices: Vec<Matrix>,
    coord_index: CoordIndex,
) -> Result<CoreProblem, MultiFormatError> {
    ValidationContext::new(&api_problem, Some(&matrices), &coord_index).validate()?;

    let mut state_registry = StateKeyRegistry::default();
    let mut extras = ExtrasBuilder::new(&mut state_registry).build().map_err(to_multi_format_error)?;

    extras.set_coord_index(coord_index);

    let coord_index = extras.get_coord_index().expect("cannot get coord index");
    let mut job_index = JobIndex::default();

    let props = get_problem_properties(&api_problem, &matrices);
    let mut blocks = get_problem_blocks(&api_problem, matrices, coord_index, &mut job_index, &props)?;

    extras.set_job_index(job_index);
    blocks.job_index = extras.get_job_index();

    let goal = Arc::new(
        create_goal_context(&api_problem, &blocks, &props, &extras, &mut state_registry)
            .map_err(to_multi_format_error)?,
    );

    let ProblemBlocks { jobs, fleet, transport, activity, locks, reserved_times_index, .. } = blocks;

    if let Some(config) = create_cluster_config(&api_problem).map_err(to_multi_format_error)? {
        extras.set_cluster_config(config);
    }

    if !reserved_times_index.is_empty() {
        extras.set_reserved_times(reserved_times_index);
    }

    Ok(CoreProblem { fleet, jobs, locks, goal, activity, transport, extras: Arc::new(extras) })
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
            let type_id = actor.vehicle.dimens.get_vehicle_type().unwrap().clone();
            let shift_idx = actor.vehicle.dimens.get_shift_index().copied().unwrap();

            let times = breaks_map
                .get(&(type_id, shift_idx))
                .iter()
                .flat_map(|data| data.iter())
                .map(|(_, _, time, duration)| {
                    let time = match &time {
                        VehicleRequiredBreakTime::ExactTime { earliest, latest } => {
                            TimeSpan::Window(TimeWindow::new(parse_time(earliest), parse_time(latest)))
                        }
                        VehicleRequiredBreakTime::OffsetTime { earliest, latest } => {
                            TimeSpan::Offset(TimeOffset::new(*earliest, *latest))
                        }
                    };
                    let duration = *duration;

                    ReservedTimeSpan { time, duration }
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

fn to_multi_format_error(error: GenericError) -> MultiFormatError {
    vec![FormatError::new(
        "E0000".to_string(),
        "cannot create vrp variant".to_string(),
        format!("need to analyze how features are defined: '{error}'"),
    )]
    .into()
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
    let has_skills = api_problem.plan.jobs.iter().any(|job| job.skills.is_some());

    let shift_has_fn = |shift_has: fn(&VehicleShift) -> bool| {
        api_problem.fleet.vehicles.iter().any(|t| t.shifts.iter().any(shift_has))
    };

    let has_breaks = shift_has_fn(|s| s.breaks.as_ref().map_or(false, |b| !b.is_empty()));
    let has_reloads = shift_has_fn(|s| s.reloads.as_ref().map_or(false, |r| !r.is_empty()));
    let has_recharges = shift_has_fn(|s| s.recharges.as_ref().is_some());

    let has_order = api_problem
        .plan
        .jobs
        .iter()
        .flat_map(get_job_tasks)
        .filter_map(|job_task| job_task.order)
        .any(|order| order > 0);

    let has_group = api_problem.plan.jobs.iter().any(|job| job.group.is_some());
    let has_value = api_problem.plan.jobs.iter().filter_map(|job| job.value).any(|value| value != 0.);
    let has_compatibility = api_problem.plan.jobs.iter().any(|job| job.compatibility.is_some());
    let has_tour_size_limits =
        api_problem.fleet.vehicles.iter().any(|v| v.limits.as_ref().map_or(false, |l| l.tour_size.is_some()));

    let has_tour_travel_limits = api_problem
        .fleet
        .vehicles
        .iter()
        .any(|v| v.limits.as_ref().map_or(false, |l| l.max_duration.or(l.max_distance).is_some()));

    ProblemProperties {
        has_multi_dimen_capacity,
        has_breaks,
        has_skills,
        has_unreachable_locations,
        has_reloads,
        has_recharges,
        has_order,
        has_group,
        has_value,
        has_compatibility,
        has_tour_size_limits,
        has_tour_travel_limits,
    }
}

fn get_problem_blocks(
    api_problem: &ApiProblem,
    matrices: Vec<Matrix>,
    coord_index: Arc<CoordIndex>,
    job_index: &mut JobIndex,
    problem_props: &ProblemProperties,
) -> Result<ProblemBlocks, MultiFormatError> {
    let fleet = read_fleet(api_problem, problem_props, &coord_index);
    let reserved_times_index = read_reserved_times_index(api_problem, &fleet);

    let transport = create_transport_costs(api_problem, &matrices, coord_index.clone()).map_err(|err| {
        vec![FormatError::new(
            "E0002".to_string(),
            "cannot create transport costs".to_string(),
            format!("check matrix routing data: '{err}'"),
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
                    format!("check fleet definition: '{err}'"),
                )]
            })
            .map::<(Arc<dyn TransportCost + Send + Sync>, Arc<dyn ActivityCost + Send + Sync>), _>(
                |(transport, activity)| (Arc::new(transport), Arc::new(activity)),
            )?
    };

    // TODO pass random from outside as there might be need to have it initialized with seed
    //      at the moment, this random instance is used only by multi job permutation generator
    let random: Arc<dyn Random + Send + Sync> = Arc::new(DefaultRandom::default());
    let (jobs, locks) = read_jobs_with_extra_locks(
        api_problem,
        problem_props,
        &coord_index,
        &fleet,
        transport.as_ref(),
        job_index,
        &random,
    );
    let locks = locks.into_iter().chain(read_locks(api_problem, job_index)).collect::<Vec<_>>();

    Ok(ProblemBlocks {
        jobs: Arc::new(jobs),
        fleet: Arc::new(fleet),
        job_index: None,
        transport,
        activity,
        locks,
        reserved_times_index,
    })
}
