use self::ActivityType::*;
use super::*;
use crate::construction::enablers::route_intervals;
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{create_route_context_with_activities, test_activity};
use crate::models::common::*;
use crate::models::problem::{Fleet, Vehicle, VehicleDetail};

const VIOLATION_CODE: ViolationCode = 1;
const RESOURCE_KEY: StateKey = 1;
const INTERVALS_KEY: StateKey = 2;

fn create_usage_activity(demand: i32) -> Activity {
    let demand = create_simple_demand(-demand);
    let single = test_single_with_simple_demand(demand);

    Activity { job: Some(single), ..test_activity() }
}

fn create_resource_activity(capacity: i32, resource_id: Option<SharedResourceId>) -> Activity {
    let mut single = test_single();
    if let Some(resource_id) = resource_id {
        single.dimens.set_value("resource_id", resource_id);
    }
    single.dimens.set_capacity(SingleDimLoad::new(capacity));

    Activity { job: Some(Arc::new(single)), ..test_activity() }
}

fn create_feature(total_jobs: usize) -> Feature {
    create_shared_resource_feature::<SingleDimLoad>(
        "shared_resource",
        total_jobs,
        VIOLATION_CODE,
        RESOURCE_KEY,
        create_interval_fn(),
        Arc::new(|activity| {
            activity.job.as_ref().and_then(|job| {
                job.dimens.get_capacity().cloned().zip(job.dimens.get_value::<SharedResourceId>("resource_id").cloned())
            })
        }),
        create_resource_demand_fn(),
    )
    .unwrap()
}

fn create_ovrp_vehicle(id: &str) -> Vehicle {
    VehicleBuilder::default().id(id).details(vec![VehicleDetail { end: None, ..test_vehicle_detail() }]).build()
}

fn create_route_ctx(
    fleet: &Fleet,
    vehicle_id: &str,
    resources: &HashMap<usize, i32>,
    activities: &[ActivityType],
) -> RouteContext {
    let activities = activities
        .iter()
        .map(|activity_type| match activity_type {
            SharedResource(resource_id) => {
                create_resource_activity(*resources.get(resource_id).unwrap(), Some(*resource_id))
            }
            NormalResource(capacity) => create_resource_activity(*capacity, None),
            Usage(demand) => create_usage_activity(*demand),
        })
        .collect();

    let mut route_ctx = create_route_context_with_activities(fleet, vehicle_id, activities);
    let intervals = route_intervals(&route_ctx.route, |activity| {
        activity.job.as_ref().map_or(false, |job| {
            let capacity: Option<&SingleDimLoad> = job.dimens.get_capacity();
            capacity.is_some()
        })
    });
    route_ctx.state_mut().put_route_state(INTERVALS_KEY, intervals);

    route_ctx
}

fn create_solution_ctx(
    resources: Vec<(usize, i32)>,
    activities: Vec<Vec<ActivityType>>,
    is_ovrp: bool,
) -> SolutionContext {
    let resources = resources.into_iter().collect::<HashMap<usize, _>>();
    let (v1, v2) = if is_ovrp {
        (create_ovrp_vehicle("v1"), create_ovrp_vehicle("v2"))
    } else {
        (test_vehicle_with_id("v1"), test_vehicle_with_id("v2"))
    };

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(v1).add_vehicle(v2).build();

    let routes = activities
        .into_iter()
        .enumerate()
        .map(|(idx, activities)| create_route_ctx(&fleet, format!("v{}", idx + 1).as_str(), &resources, &activities))
        .collect();

    SolutionContext { routes, ..create_empty_solution_context() }
}

fn create_interval_fn() -> SharedResourceIntervalFn {
    Arc::new(move |route_ctx| route_ctx.state.get_route_state::<Vec<(usize, usize)>>(INTERVALS_KEY))
}

fn create_resource_demand_fn() -> SharedResourceDemandFn<SingleDimLoad> {
    Arc::new(|single| single.dimens.get_demand().map(|demand| demand.delivery.0))
}

enum ActivityType {
    Usage(i32),
    NormalResource(i32),
    SharedResource(usize),
}

parameterized_test! {can_update_resource_consumption, (resources, activities, total_jobs, expected_resources), {
    can_update_resource_consumption_impl(resources, activities, total_jobs, expected_resources);
}}

can_update_resource_consumption! {
    case_01_single_shared_resource: (vec![(0, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), SharedResource(0), Usage(2)]],
        None,
        vec![vec![None, None, Some(6), None, None], vec![None, None, Some(6), None, None]],
    ),

    case_02_two_shared_resources: (vec![(0, 10), (1, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), SharedResource(1), Usage(1)]],
        None,
        vec![vec![None, None, Some(8), None, None], vec![None, None, Some(9), None, None]],
    ),

    case_03_mixed_normal_resource: (vec![(0, 10), (1, 5)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), NormalResource(10), Usage(2)]],
        None,
        vec![vec![None, None, Some(8), None, None], vec![None, None, None, None, None]],
    ),

    case_04_partial_solution: (vec![(0, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), SharedResource(0), Usage(2)]],
        Some(100),
        vec![vec![None, None, None, None, None], vec![None, None, None, None, None]],
    ),
}

fn can_update_resource_consumption_impl(
    resources: Vec<(usize, i32)>,
    activities: Vec<Vec<ActivityType>>,
    total_jobs: Option<usize>,
    expected_resources: Vec<Vec<Option<i32>>>,
) {
    let total_jobs = total_jobs.unwrap_or(activities[0].len() + activities[1].len());
    let mut solution_ctx = create_solution_ctx(resources, activities, false);
    let state = create_feature(total_jobs).state.unwrap();

    state.accept_solution_state(&mut solution_ctx);

    let actual_resources = solution_ctx
        .routes
        .iter()
        .map(|route_ctx| {
            route_ctx
                .route
                .tour
                .all_activities()
                .map(|activity| {
                    route_ctx
                        .state
                        .get_activity_state::<SingleDimLoad>(RESOURCE_KEY, activity)
                        .map(|resource| resource.value)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_resources, expected_resources);
}

parameterized_test! {can_constraint_route, (resources, activities, total_jobs, job_demand, expected), {
    can_constraint_route_impl(resources, activities, total_jobs, job_demand, expected);
}}

can_constraint_route! {
    case_01_partial_solution: (vec![(0, 10)],
        vec![Usage(2), SharedResource(0), Usage(2)], 1, Some(1), Some(VIOLATION_CODE),
    ),

    case_02_complete_solution: (vec![(0, 10)],
        vec![Usage(2), SharedResource(0), Usage(2)], 3, Some(1), None,
    ),

    case_03_no_demand: (vec![(0, 10)],
        vec![Usage(2), SharedResource(0), Usage(2)], 3, None, None,
    ),

    case_04_no_resource: (vec![(0, 10)],
        vec![Usage(2), Usage(2)], 2, Some(1), None,
    ),
}

fn can_constraint_route_impl(
    resources: Vec<(usize, i32)>,
    activities: Vec<ActivityType>,
    total_jobs: usize,
    job_demand: Option<i32>,
    expected: Option<i32>,
) {
    let job = Job::Single(job_demand.map_or_else(
        || test_single_with_id("job1"),
        |demand| test_single_with_simple_demand(create_simple_demand(-demand)),
    ));
    let constraint = create_feature(total_jobs).constraint.unwrap();
    let solution_ctx = create_solution_ctx(resources, vec![activities], false);

    let result = constraint.evaluate(&MoveContext::route(&solution_ctx, &solution_ctx.routes[0], &job));

    assert_eq!(result.map(|result| result.code), expected);
}

parameterized_test! {can_constraint_activity, (resources, activities, insertion_idx, is_ovrp, job_demand, expected), {
    can_constraint_activity_impl(resources, activities, insertion_idx, is_ovrp, job_demand, expected);
}}

can_constraint_activity! {
    case_01_enough_resource: (vec![(0, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(0), Usage(6)]],
        0, false, Some(1), None,
    ),
    case_02_enough_resource: (vec![(0, 10), (1, 10)],
        vec![vec![SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(1), Usage(6)]],
        2, false, Some(1), None,
    ),

    case_03_not_enough_resource: (vec![(0, 10)],
        vec![vec![SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(0), Usage(6)]],
        2, false, Some(1), Some(VIOLATION_CODE),
    ),
    case_04_not_enough_resource: (vec![(0, 10)],
        vec![vec![SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(0), Usage(5)]],
        2, false, Some(2), Some(VIOLATION_CODE),
    ),

    case_05_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![Usage(1), SharedResource(0), Usage(1)], vec![Usage(6)]],
        1, true, Some(1), None,
    ),
    case_06_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![Usage(1), SharedResource(0), Usage(1)], vec![Usage(6)]],
        2, true, Some(1), Some(VIOLATION_CODE),
    ),
    case_07_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![Usage(1), SharedResource(0), Usage(1)], vec![Usage(6)]],
        3, true, Some(1), Some(VIOLATION_CODE),
    ),

    case_08_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![SharedResource(0)], vec![Usage(1), SharedResource(0), Usage(1)]],
        0, true, Some(1), None,
    ),
    case_09_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![SharedResource(0)], vec![Usage(1), SharedResource(0), Usage(1)]],
        1, true, Some(1), Some(VIOLATION_CODE),
    ),
}

fn can_constraint_activity_impl(
    resources: Vec<(usize, i32)>,
    activities: Vec<Vec<ActivityType>>,
    insertion_idx: usize,
    is_ovrp: bool,
    demand: Option<i32>,
    expected: Option<i32>,
) {
    let target = demand.map_or_else(test_activity, create_usage_activity);
    let total_jobs = activities[0].len() + activities[1].len();
    let mut solution_ctx = create_solution_ctx(resources, activities, is_ovrp);
    let feature = create_feature(total_jobs);
    feature.state.unwrap().accept_solution_state(&mut solution_ctx);
    let activity_ctx =
        ActivityContext { index: insertion_idx, prev: &create_usage_activity(0), target: &target, next: None };

    let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(&solution_ctx.routes[0], &activity_ctx));

    assert_eq!(result.map(|result| result.code), expected)
}
