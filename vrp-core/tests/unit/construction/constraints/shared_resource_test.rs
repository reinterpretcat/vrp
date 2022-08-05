use self::ActivityType::*;
use super::*;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{create_route_context_with_activities, test_activity};
use crate::models::common::*;
use crate::models::problem::Fleet;

fn create_terminal_activity() -> Activity {
    Activity { job: None, ..test_activity() }
}

fn create_usage_activity(demand: i32) -> Activity {
    let demand = SingleDimLoad::new(demand);
    let single = test_single_with_simple_demand(Demand::<SingleDimLoad> {
        pickup: (Default::default(), Default::default()),
        delivery: (demand, Default::default()),
    });

    Activity { job: Some(single), ..test_activity() }
}

fn create_resource_activity(capacity: i32, resource_id: SharedResourceId) -> Activity {
    let mut single = test_single();
    single.dimens.set_value("resource_id", resource_id);
    single.dimens.set_capacity(SingleDimLoad::new(capacity));

    Activity { job: Some(Arc::new(single)), ..test_activity() }
}

fn create_shared_resource_model(total_jobs: usize) -> SharedResourceModule<SingleDimLoad> {
    let code = 1;
    let resource_key = 1;
    let intervals_key = 2;

    SharedResourceModule::new(
        total_jobs,
        code,
        resource_key,
        Arc::new(move |route_ctx| route_ctx.state.get_route_state::<Vec<(usize, usize)>>(intervals_key)),
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
            Terminal => create_terminal_activity(),
            Resource(resource_id) => {
                create_resource_activity(resources.get(&resource_id).unwrap().clone(), resource_id)
            }
            Usage(demand) => create_usage_activity(demand),
        })
        .collect();
    create_route_context_with_activities(&fleet, vehicle_id, activities)
}

enum ActivityType {
    Terminal,
    Usage(i32),
    Resource(usize),
}

#[test]
fn can_update_resource_consumption() {
    let resources = [(0, 10), (1, 5)].into_iter().collect::<HashMap<usize, _>>();
    let v1_activities = vec![Terminal, Usage(2), Resource(0), Usage(2)];

    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .add_vehicle(test_vehicle_with_id("v3"))
        .build();

    let _solution_ctx = SolutionContext {
        routes: vec![create_route_ctx(&fleet, "v1", &resources, v1_activities)],
        ..create_empty_solution_context()
    };
}
