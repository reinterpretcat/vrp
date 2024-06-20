use super::*;
use crate::construction::features::capacity::JobDemandDimension;
use crate::helpers::construction::heuristics::create_state_key;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;

struct TestFastServiceAspects {
    state_key: StateKey,
}

impl FastServiceAspects for TestFastServiceAspects {
    fn get_state_key(&self) -> StateKey {
        self.state_key
    }

    fn get_demand_type(&self, single: &Single) -> Option<DemandType> {
        single.dimens.get_job_demand().map(|demand: &Demand<SingleDimLoad>| demand.get_type())
    }
}

fn create_test_feature(route_intervals: RouteIntervals) -> Feature {
    create_fast_service_feature::<_>(
        "fast_service",
        TestTransportCost::new_shared(),
        TestActivityCost::new_shared(),
        route_intervals,
        None,
        TestFastServiceAspects { state_key: create_state_key() },
    )
    .unwrap()
}

fn create_test_feature_no_reload() -> Feature {
    create_test_feature(RouteIntervals::Single)
}

mod local_estimation {
    use super::*;
    use crate::helpers::construction::features::{create_simple_demand, create_simple_dynamic_demand};
    use crate::models::solution::Activity;
    use std::iter::once;

    fn run_estimation_test_case<T>(test_case: InsertionTestCase<T>, job: Arc<Single>, activities: Vec<Activity>) {
        let InsertionTestCase { target_index, target_location, end_time, expected_cost, .. } = test_case;
        let (objective, state) = {
            let feature = create_test_feature_no_reload();
            (feature.objective.unwrap(), feature.state.unwrap())
        };
        let mut route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_start(ActivityBuilder::default().job(None).build())
                    .with_end(ActivityBuilder::default().job(None).schedule(Schedule::new(end_time, end_time)).build())
                    .add_activities(activities)
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

    struct InsertionTestCase<T> {
        target_index: usize,
        target_location: Location,
        demand: i32,
        activities: Vec<T>,
        end_time: Timestamp,
        expected_cost: Cost,
    }

    parameterized_test! {can_estimate_single_job_insertion_without_reload, test_case_data, {
        can_estimate_single_job_insertion_without_reload_impl(test_case_data);
    }}

    can_estimate_single_job_insertion_without_reload! {
        case01_delivery_deviate_route: InsertionTestCase {
            target_index: 1, target_location: 15, demand: -1, activities: vec![10, 20], end_time: 40., expected_cost: 15.,
        },
        case02_delivery_along_route: InsertionTestCase {
            target_index: 2, target_location: 15, demand: -1, activities: vec![10, 20], end_time: 40., expected_cost: 15.,
        },

        case03_pickup_deviate_route: InsertionTestCase {
            target_index: 1, target_location: 15, demand: 1, activities: vec![10, 20], end_time: 40., expected_cost: 35.,
        },
        case04_pickup_along_route: InsertionTestCase {
            target_index: 2, target_location: 15, demand: 1, activities: vec![10, 20], end_time: 40., expected_cost: 25.,
        },
    }

    fn can_estimate_single_job_insertion_without_reload_impl(test_case: InsertionTestCase<Location>) {
        let job = SingleBuilder::default()
            .location(Some(test_case.target_location))
            .demand(create_simple_demand(test_case.demand))
            .build_shared();
        let activities = test_case.activities.iter().map(|l| ActivityBuilder::with_location(*l).build()).collect();

        run_estimation_test_case(test_case, job, activities);
    }

    parameterized_test! {can_estimate_multi_job_insertion_without_reload, test_case_data, {
        can_estimate_multi_job_insertion_without_reload_impl(test_case_data);
    }}

    can_estimate_multi_job_insertion_without_reload! {
        case01_start_next_activity: InsertionTestCase {
            target_index: 1, target_location: 15, demand: 1, activities: vec![(10, Some(-1)), (20, None)], end_time: 40., expected_cost: 5.,
        },
        case02_start_skip_activity: InsertionTestCase {
            target_index: 1, target_location: 15, demand: 1, activities: vec![(10, None), (20, Some(-1))], end_time: 40., expected_cost: 15.,
        },

        case03_end_prev_activity: InsertionTestCase {
            target_index: 3, target_location: 15, demand: -1, activities: vec![(10, Some(1)), (20, None)], end_time: 40., expected_cost: 15.,
        },
        case04_end_prev_activity: InsertionTestCase {
            target_index: 2, target_location: 15, demand: -1, activities: vec![(10, Some(1)), (20, None)], end_time: 40., expected_cost: 5.,
        },
    }

    fn can_estimate_multi_job_insertion_without_reload_impl(test_case: InsertionTestCase<(Location, Option<i32>)>) {
        let job = SingleBuilder::default()
            .location(Some(test_case.target_location))
            .demand(create_simple_dynamic_demand(test_case.demand))
            .build_shared();
        let jobs = test_case.activities.iter().filter_map(|(l, demand)| demand.map(|d| (l, d))).map(|(l, d)| {
            SingleBuilder::default().location(Some(*l)).demand(create_simple_dynamic_demand(d)).build_shared()
        });
        let jobs = once(job).chain(jobs).collect::<Vec<_>>();
        let multi = Multi::new_shared(jobs, Default::default());
        let activities = test_case
            .activities
            .iter()
            .fold((1, Vec::default()), |(idx, mut activities), (l, demand)| {
                let (idx, activity) = if demand.is_some() {
                    let job = multi.jobs[idx].clone();
                    let activity = ActivityBuilder::with_location(*l).job(Some(job)).build();
                    (idx + 1, activity)
                } else {
                    (idx, ActivityBuilder::with_location(*l).build())
                };
                activities.push(activity);

                (idx, activities)
            })
            .1;

        run_estimation_test_case(test_case, multi.jobs[0].clone(), activities);
        drop(multi);
    }
}

mod global_estimation {
    use super::*;
    use crate::construction::enablers::get_route_intervals;
    use crate::helpers::construction::heuristics::InsertionContextBuilder;

    #[test]
    fn can_get_solution_fitness() {
        let objective = create_test_feature_no_reload().objective.expect("no objective");
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .add_activity(ActivityBuilder::with_location(10).build())
                    .add_activity(ActivityBuilder::with_location(20).build())
                    .build(),
            )
            .build();
        let insertion_ctx = InsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

        let fitness = objective.fitness(&insertion_ctx);

        assert_eq!(fitness, 30.)
    }

    #[test]
    fn can_get_solution_fitness_with_reload() {
        const INTERVAL_LOCATION: Location = 15;
        let state_key = create_state_key();

        let route_intervals = RouteIntervals::Multiple {
            is_marker_single_fn: Arc::new(|single| single.places.iter().any(|p| p.location == Some(INTERVAL_LOCATION))),
            is_new_interval_needed_fn: Arc::new(|_| unreachable!()),
            is_obsolete_interval_fn: Arc::new(|_, _, _| unreachable!()),
            is_assignable_fn: Arc::new(|_, _| unreachable!()),
            intervals_key: state_key,
        };
        let objective = create_test_feature(route_intervals).objective.expect("no objective");
        let route = RouteBuilder::default()
            .add_activity(ActivityBuilder::with_location(10).build())
            .add_activity(ActivityBuilder::with_location(INTERVAL_LOCATION).build())
            .add_activity(ActivityBuilder::with_location(20).build())
            .build();
        let state = RouteStateBuilder::default()
            .add_route_state(
                state_key,
                get_route_intervals(&route, |activity| activity.place.location == INTERVAL_LOCATION),
            )
            .build();
        let route_ctx = RouteContextBuilder::default().with_route(route).with_state(state).build();
        let insertion_ctx = InsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

        let fitness = objective.fitness(&insertion_ctx);

        assert_eq!(fitness, 15.)
    }
}
