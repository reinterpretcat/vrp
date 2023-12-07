use super::*;
use crate::construction::enablers::NoRouteIntervals;
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;

const STATE_KEY: StateKey = 2;

fn create_test_feature(route_intervals: Arc<dyn RouteIntervals + Send + Sync>) -> Feature {
    create_fast_service_feature::<SingleDimLoad>(
        "fast_service",
        TestTransportCost::new_shared(),
        TestActivityCost::new_shared(),
        route_intervals,
        STATE_KEY,
    )
    .unwrap()
}

fn create_test_feature_no_reload() -> Feature {
    create_test_feature(Arc::new(NoRouteIntervals::default()))
}

struct InsertionTestCase<T: LoadOps> {
    target_index: usize,
    target_location: Location,
    demand: Demand<T>,
    activities: Vec<Location>,
    end_time: Timestamp,
    expected_cost: Cost,
}

parameterized_test! {can_estimate_single_job_insertion_without_reload, test_case_data, {
    can_estimate_single_job_insertion_without_reload_impl(test_case_data);
}}

can_estimate_single_job_insertion_without_reload! {
    case01_delivery_deviate_route: InsertionTestCase {
        target_index: 1, target_location: 15, demand: create_simple_demand(-1), activities: vec![10, 20], end_time: 40., expected_cost: 15.,
    },
    case02_delivery_along_route: InsertionTestCase {
        target_index: 2, target_location: 15, demand: create_simple_demand(-1), activities: vec![10, 20], end_time: 40., expected_cost: 15.,
    },

    case03_pickup_deviate_route: InsertionTestCase {
        target_index: 1, target_location: 15, demand: create_simple_demand(1), activities: vec![10, 20], end_time: 40., expected_cost: 35.,
    },
    case04_pickup_along_route: InsertionTestCase {
        target_index: 2, target_location: 15, demand: create_simple_demand(1), activities: vec![10, 20], end_time: 40., expected_cost: 25.,
    },
}

fn can_estimate_single_job_insertion_without_reload_impl<T: LoadOps>(test_case: InsertionTestCase<T>) {
    let InsertionTestCase { target_index, target_location, demand, activities, end_time, expected_cost } = test_case;
    let job = SingleBuilder::default().location(Some(target_location)).demand(demand).build_shared();
    let (objective, state) = {
        let feature = create_test_feature_no_reload();
        (feature.objective.unwrap(), feature.state.unwrap())
    };
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_start(ActivityBuilder::default().job(None).build())
                .with_end(ActivityBuilder::default().job(None).schedule(Schedule::new(end_time, end_time)).build())
                .add_activities(activities.into_iter().map(|l| ActivityBuilder::with_location(l).build()))
                .build(),
        )
        .build();
    state.accept_route_state(&mut route_ctx);
    let activity_ctx = ActivityContext {
        index: target_index,
        prev: route_ctx.route().tour.get(target_index - 1).unwrap(),
        target: &ActivityBuilder::with_location(target_location).job(Some(job)).build(),
        next: route_ctx.route().tour.get(target_index),
    };

    let result = objective.estimate(&MoveContext::activity(&route_ctx, &activity_ctx));

    assert_eq!(result, expected_cost);
}
