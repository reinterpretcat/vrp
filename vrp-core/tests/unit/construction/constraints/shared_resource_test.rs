use self::ActivityType::*;
use super::*;
use crate::construction::extensions::route_intervals;
use crate::helpers::construction::constraints::create_simple_demand;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{create_route_context_with_activities, test_activity};
use crate::models::common::*;
use crate::models::problem::Fleet;

const CODE: i32 = 1;
const RESOURCE_KEY: i32 = 1;
const INTERVALS_KEY: i32 = 2;

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

fn create_shared_resource_model(total_jobs: usize) -> SharedResourceModule<SingleDimLoad> {
    SharedResourceModule::new(
        total_jobs,
        CODE,
        RESOURCE_KEY,
        Arc::new(move |route_ctx| route_ctx.state.get_route_state::<Vec<(usize, usize)>>(INTERVALS_KEY)),
        Arc::new(|activity| {
            activity.job.as_ref().and_then(|job| {
                job.dimens.get_capacity().cloned().zip(job.dimens.get_value::<SharedResourceId>("resource_id").cloned())
            })
        }),
        Arc::new(|single| single.dimens.get_demand().map(|demand| demand.delivery.0)),
    )
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

fn create_solution_ctx(resources: Vec<(usize, i32)>, activities: Vec<Vec<ActivityType>>) -> SolutionContext {
    let resources = resources.into_iter().collect::<HashMap<usize, _>>();
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .build();

    let routes = activities
        .into_iter()
        .enumerate()
        .map(|(idx, activities)| create_route_ctx(&fleet, format!("v{}", idx + 1).as_str(), &resources, &activities))
        .collect();

    SolutionContext { routes, ..create_empty_solution_context() }
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
    let mut solution_ctx = create_solution_ctx(resources, activities);
    let shared_resource_module = create_shared_resource_model(total_jobs);

    shared_resource_module.accept_solution_state(&mut solution_ctx);

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
        vec![Usage(2), SharedResource(0), Usage(2)], 1, Some(1), Some(CODE),
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
    let constraint = SharedResourceHardRouteConstraint::<SingleDimLoad> {
        code: CODE,
        total_jobs,
        interval_fn: Arc::new(move |route_ctx| route_ctx.state.get_route_state::<Vec<(usize, usize)>>(INTERVALS_KEY)),
        resource_demand_fn: Arc::new(|single| single.dimens.get_demand().map(|demand| demand.delivery.0)),
    };
    let solution_ctx = create_solution_ctx(resources, vec![activities]);

    let result = constraint.evaluate_job(&solution_ctx, &solution_ctx.routes[0], &job);

    assert_eq!(result.map(|result| result.code), expected);
}
