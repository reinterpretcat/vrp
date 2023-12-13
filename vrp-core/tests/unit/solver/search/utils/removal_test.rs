use super::*;
use crate::helpers::models::domain::{create_empty_solution_context, create_registry_context};
use crate::helpers::models::problem::{test_driver, test_vehicle_with_id, FleetBuilder, SingleBuilder};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Dimensions;
use crate::models::problem::{Fleet, Multi};
use rosomaxa::utils::DefaultRandom;
use std::iter::once;

fn create_route_with_jobs_activities(
    fleet: &Fleet,
    vehicle_idx: usize,
    jobs: usize,
    activities: usize,
) -> RouteContext {
    assert!(jobs > 0);
    assert!(activities >= jobs);

    let vehicle = format!("v{}", vehicle_idx + 1);
    let activities_per_job = activities / jobs;
    let left_overs = activities - activities_per_job * jobs;
    let get_activity = |job_idx: usize| {
        ActivityBuilder::default()
            .job(Some(SingleBuilder::default().id(format!("{job_idx}").as_str()).build_shared()))
            .build()
    };
    // NOTE need to keep multi-jobs somewhere to keep weak reference in sub-jobs alive
    let mut multi_jobs = Vec::new();

    let activities = (0..jobs)
        .flat_map(|job_idx| {
            if activities_per_job > 1 {
                let singles = (0..activities_per_job)
                    .map(|activity_idx| {
                        SingleBuilder::default().id(format!("{job_idx}_{activity_idx}").as_str()).build_shared()
                    })
                    .collect::<Vec<_>>();
                let multi = Multi::new_shared(singles, Dimensions::default());
                multi_jobs.push(multi.clone());
                multi
                    .jobs
                    .iter()
                    .cloned()
                    .map(|single| ActivityBuilder::default().job(Some(single)).build())
                    .collect::<Vec<_>>()
                    .into_iter()
            } else {
                once(get_activity(job_idx)).collect::<Vec<_>>().into_iter()
            }
        })
        .chain((0..left_overs).map(|idx| get_activity(jobs + idx)));

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(fleet, vehicle.as_str()).add_activities(activities).build())
        .build();

    route_ctx.state_mut().put_route_state(StateKey(0), multi_jobs);

    route_ctx
}

fn create_fleet(vehicles: usize) -> Fleet {
    FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles((0..vehicles).map(|idx| test_vehicle_with_id(format!("v{}", idx + 1).as_str())).collect())
        .build()
}

fn create_solution_ctx(fleet: &Fleet, routes: Vec<(usize, usize)>) -> SolutionContext {
    let mut registry = create_registry_context(fleet);
    let routes = routes
        .into_iter()
        .enumerate()
        .map(|(idx, (jobs, activities))| create_route_with_jobs_activities(fleet, idx, jobs, activities))
        .collect::<Vec<_>>();
    routes.iter().for_each(|route_ctx| {
        let _ = registry.get_route(route_ctx.route().actor.as_ref());
    });

    SolutionContext { routes, registry, ..create_empty_solution_context() }
}

fn get_job_from_solution_ctx(solution_ctx: &SolutionContext, route_idx: usize, activity_idx: usize) -> Job {
    solution_ctx.routes.get(route_idx).unwrap().route().tour.get(activity_idx).unwrap().retrieve_job().unwrap()
}

parameterized_test! {can_try_remove_job_with_job_limit, (jobs_activities, limits, expected_removed_activities), {
    can_try_remove_job_with_job_limit_impl(jobs_activities, limits, expected_removed_activities);
}}

can_try_remove_job_with_job_limit! {
    case_01: ((10, 10), (20, 2), 1),
    case_02: ((10, 20), (20, 2), 2),
    case_03: ((10, 30), (20, 2), 3),
    case_04: ((10, 10), (1, 1), 1),
    case_05: ((10, 10), (0, 0), 0),
}

fn can_try_remove_job_with_job_limit_impl(
    jobs_activities: (usize, usize),
    limits: (usize, usize),
    expected_removed_activities: usize,
) {
    let (jobs, activities) = jobs_activities;
    let (route_idx, activity_idx) = (0, 1);
    let (ruined_activities, affected_routes) = limits;
    let limits = RemovalLimits {
        removed_activities_range: ruined_activities..ruined_activities,
        affected_routes_range: affected_routes..affected_routes,
    };

    let fleet = create_fleet(1);
    let mut solution_ctx = create_solution_ctx(&fleet, vec![(jobs, activities)]);
    let job = get_job_from_solution_ctx(&solution_ctx, route_idx, activity_idx);
    let mut removal = JobRemovalTracker::new(&limits, &DefaultRandom::default());

    let result = removal.try_remove_job(&mut solution_ctx, route_idx, &job);

    if expected_removed_activities > 0 {
        assert!(result);
        assert_eq!(solution_ctx.required.len(), 1);
        assert!(solution_ctx.required[0] == job);
        assert_eq!(solution_ctx.routes[0].route().tour.job_activity_count(), activities - expected_removed_activities);
        assert_eq!(removal.activities_left, (ruined_activities - expected_removed_activities) as i32);
        assert!(removal.removed_jobs.contains(&job));
        assert!(removal.affected_actors.contains(&solution_ctx.routes[0].route().actor));
    } else {
        assert!(!result);
        assert!(solution_ctx.required.is_empty());
        assert_eq!(solution_ctx.routes[0].route().tour.job_activity_count(), activities);
        assert_eq!(removal.activities_left, ruined_activities as i32);
        assert!(!removal.removed_jobs.contains(&job));
        assert!(!removal.affected_actors.contains(&solution_ctx.routes[0].route().actor));
    }
}

parameterized_test! {can_try_remove_route_with_limit, (jobs_activities, limits, is_random_hit, expected_affected), {
    can_try_remove_route_with_limit_impl(jobs_activities, limits, is_random_hit, expected_affected);
}}

can_try_remove_route_with_limit! {
    case_01_one_route_left: ((10, 10), (10, 1), false, (10, 10, 1, 0)),
    case_02_no_routes_left: ((10, 10), (10, 0), false, (0, 0, 0, 1)),
    case_03_partial_remove: ((10, 10), (9, 1), false, (9, 9, 1, 1)),
    case_04_fully_remove_by_hit: ((10, 10), (9, 1), true, (10, 10, 1, 0)),
}

fn can_try_remove_route_with_limit_impl(
    jobs_activities: (usize, usize),
    limits: (usize, usize),
    is_random_hit: bool,
    expected_affected: (usize, usize, usize, usize),
) {
    let (jobs, activities) = jobs_activities;
    let (ruined_activities, affected_routes) = limits;
    let limits = RemovalLimits {
        removed_activities_range: ruined_activities..ruined_activities,
        affected_routes_range: affected_routes..affected_routes,
    };
    let route_idx = 0;
    let fleet = create_fleet(1);
    let mut solution_ctx = create_solution_ctx(&fleet, vec![(jobs, activities)]);
    let actor = solution_ctx.routes[0].route().actor.clone();
    let random = FakeRandom::new(vec![], vec![if is_random_hit { 0. } else { 10. }]);
    let mut removal = JobRemovalTracker::new(&limits, &DefaultRandom::default());

    let result = removal.try_remove_route(&mut solution_ctx, route_idx, &random);

    let (expected_affected_activities, expected_affected_jobs, expected_affected_routes, expected_result_routes) =
        expected_affected;
    if expected_affected_routes == 1 {
        assert!(result);
        assert_eq!(removal.activities_left, (ruined_activities as i32 - expected_affected_activities as i32).max(0));
        assert_eq!(removal.routes_left, (affected_routes - expected_affected_routes) as i32);
        assert_eq!(solution_ctx.required.len(), expected_affected_jobs);
        assert_eq!(solution_ctx.routes.len(), expected_result_routes);
        assert_eq!(solution_ctx.registry.next_route().count(), 1 - expected_result_routes);
        assert_eq!(removal.removed_jobs.len(), solution_ctx.required.len());
        assert!(removal.affected_actors.contains(&actor));
    } else {
        assert!(!result);
        assert!(solution_ctx.required.is_empty());
        assert_eq!(solution_ctx.routes.len(), 1);
        assert_eq!(solution_ctx.routes[0].route().tour.jobs().count(), jobs);
        assert_eq!(solution_ctx.registry.next_route().count(), 0);
        assert!(!removal.affected_actors.contains(&actor));
    }
}

parameterized_test! {can_detect_limit_reached, (ruined_activities, affected_routes, expected), {
    can_detect_limit_reached_impl(ruined_activities, affected_routes, expected);
}}

can_detect_limit_reached! {
    case_01: (1, 1, false),
    case_02: (0, 1, true),
    case_03: (0, 0, true),
}

fn can_detect_limit_reached_impl(ruined_activities: usize, affected_routes: usize, expected: bool) {
    let limits = RemovalLimits {
        removed_activities_range: ruined_activities..ruined_activities,
        affected_routes_range: affected_routes..affected_routes,
    };

    let removal = JobRemovalTracker::new(&limits, &DefaultRandom::default());

    assert_eq!(removal.is_limit(), expected);
}
