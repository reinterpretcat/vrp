use self::ActivityType::*;
use super::*;
use crate::construction::extensions::route_intervals;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{create_route_context_with_activities, test_activity};
use crate::models::common::*;
use crate::models::problem::Fleet;

const CODE: i32 = 1;
const RESOURCE_KEY: i32 = 1;
const INTERVALS_KEY: i32 = 2;

fn create_usage_activity(demand: i32) -> Activity {
    let demand = SingleDimLoad::new(demand);
    let single = test_single_with_simple_demand(Demand::<SingleDimLoad> {
        pickup: (Default::default(), Default::default()),
        delivery: (demand, Default::default()),
    });

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
    activities: Vec<ActivityType>,
) -> RouteContext {
    let activities = activities
        .into_iter()
        .map(|activity_type| match activity_type {
            SharedResource(resource_id) => {
                create_resource_activity(resources.get(&resource_id).unwrap().clone(), Some(resource_id))
            }
            NormalResource(capacity) => create_resource_activity(capacity, None),
            Usage(demand) => create_usage_activity(demand),
        })
        .collect();

    let mut route_ctx = create_route_context_with_activities(&fleet, vehicle_id, activities);
    let intervals = route_intervals(&route_ctx.route, |activity| {
        activity.job.as_ref().map_or(true, |job| {
            let capacity: Option<&SingleDimLoad> = job.dimens.get_capacity();
            capacity.is_some()
        })
    });
    route_ctx.state_mut().put_route_state(INTERVALS_KEY, intervals);

    route_ctx
}

enum ActivityType {
    Usage(i32),
    NormalResource(i32),
    SharedResource(usize),
}

#[test]
fn can_update_resource_consumption() {
    let resources = [(0, 10), (1, 5)].into_iter().collect::<HashMap<usize, _>>();
    let v1_activities = vec![Usage(2), SharedResource(0), Usage(2)];
    let v2_activities = vec![Usage(2), SharedResource(0), Usage(2)];
    let expected_resources: Vec<Vec<Option<i32>>> = vec![];

    let total_jobs = v1_activities.len() + v2_activities.len();
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .build();
    let mut solution_ctx = SolutionContext {
        routes: vec![
            create_route_ctx(&fleet, "v1", &resources, v1_activities),
            create_route_ctx(&fleet, "v2", &resources, v2_activities),
        ],
        ..create_empty_solution_context()
    };
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
