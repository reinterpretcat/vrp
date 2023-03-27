use super::*;
use crate::construction::enablers::create_typed_actor_groups;
use crate::construction::enablers::VehicleTie;
use crate::helpers::*;
use hashbrown::HashMap;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::problem::Actor;
use vrp_core::models::problem::{Fleet, Single};

const VIOLATION_CODE: i32 = 1;
const STATE_KEY: i32 = 2;

fn get_total_jobs(routes: &[(&str, Vec<Option<&str>>)]) -> usize {
    routes.iter().map(|(_, jobs)| jobs.len()).sum::<usize>() + 1
}

fn create_test_fleet() -> Fleet {
    Fleet::new(
        vec![Arc::new(test_driver())],
        vec![Arc::new(test_vehicle("v1")), Arc::new(test_vehicle("v2"))],
        Box::new(|actors| create_typed_actor_groups(actors)),
    )
}

fn create_test_single(group: Option<&str>) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    single.dimens.set_job_group(group.map(|group| group.to_string()));

    Arc::new(single)
}

fn create_test_solution_context(
    total_jobs: usize,
    fleet: &Fleet,
    routes: Vec<(&str, Vec<Option<&str>>)>,
) -> SolutionContext {
    SolutionContext {
        required: (0..total_jobs).map(|_| Job::Single(create_test_single(None))).collect(),
        routes: routes
            .into_iter()
            .map(|(vehicle, groups)| {
                let mut state = RouteState::default();
                state.put_route_state(
                    STATE_KEY,
                    (groups.iter().filter_map(|g| *g).map(|g| g.to_string()).collect::<HashSet<_>>(), groups.len()),
                );

                RouteContext::new_with_state(
                    Arc::new(create_route_with_activities(
                        fleet,
                        vehicle,
                        groups
                            .into_iter()
                            .map(|group| create_activity_with_job_at_location(create_test_single(group), 1))
                            .collect(),
                    )),
                    Arc::new(state),
                )
            })
            .collect(),
        ..create_solution_context_for_fleet(fleet)
    }
}

fn get_actor(fleet: &Fleet, vehicle: &str) -> Arc<Actor> {
    fleet.actors.iter().find(|actor| actor.vehicle.dimens.get_vehicle_id().unwrap() == vehicle).unwrap().clone()
}

fn get_actor_groups(solution_ctx: &mut SolutionContext, state_key: i32) -> HashMap<String, Arc<Actor>> {
    solution_ctx
        .routes
        .iter()
        .filter_map(|route_ctx| {
            route_ctx
                .state
                .get_route_state::<HashSet<String>>(state_key)
                .map(|groups| (route_ctx.route().actor.clone(), groups.clone()))
        })
        .fold(HashMap::default(), |mut acc, (actor, groups)| {
            groups.into_iter().for_each(|group| {
                acc.insert(group, actor.clone());
            });
            acc
        })
}

fn compare_actor_groups(fleet: &Fleet, original: HashMap<String, Arc<Actor>>, expected: Vec<(&str, &str)>) {
    let test = expected
        .iter()
        .map(|(group, vehicle)| (group.to_string(), get_actor(fleet, vehicle)))
        .collect::<HashMap<_, _>>();

    assert_eq!(original.len(), test.len());
    assert!(original.keys().all(|k| test[k] == original[k]));
}

#[test]
fn can_build_expected_module() {
    let total_jobs = 1;
    let module = GroupModule::new(total_jobs, VIOLATION_CODE, STATE_KEY);

    assert_eq!(module.state_keys().cloned().collect::<Vec<_>>(), vec![STATE_KEY]);
    assert_eq!(module.get_constraints().count(), 1);
}

parameterized_test! {can_accept_insertion, (routes, job_group, expected), {
    can_accept_insertion_impl(routes, job_group, expected);
}}

can_accept_insertion! {
    case_01: (vec![("v1", vec![None])], Some("g1"), vec![("g1", "v1")]),
    case_02: (vec![("v1", vec![None]), ("v2", vec![Some("g2")])], Some("g1"), vec![("g1", "v1"), ("g2", "v2")]),
}

fn can_accept_insertion_impl(
    routes: Vec<(&str, Vec<Option<&str>>)>,
    job_group: Option<&str>,
    expected: Vec<(&str, &str)>,
) {
    let total_jobs = get_total_jobs(&routes) + 1;
    let fleet = create_test_fleet();
    let module = GroupModule::new(total_jobs, VIOLATION_CODE, STATE_KEY);
    let mut solution = create_test_solution_context(total_jobs, &fleet, routes);
    module.accept_solution_state(&mut solution);

    module.accept_insertion(&mut solution, 0, &Job::Single(create_test_single(job_group)));

    compare_actor_groups(&fleet, get_actor_groups(&mut solution, STATE_KEY), expected);
}

parameterized_test! {can_accept_solution_state, (routes, expected), {
    can_accept_solution_state_impl(routes, expected);
}}

can_accept_solution_state! {
    case_01: (vec![("v1", vec![Some("g1")])], vec![("g1", "v1")]),
    case_02: (vec![("v1", vec![Some("g1")]), ("v2", vec![Some("g2")])], vec![("g1", "v1"), ("g2", "v2")]),
    case_03: (vec![("v1", vec![Some("g1")]), ("v1", vec![Some("g2")])], vec![("g1", "v1"), ("g2", "v1")]),
    case_04: (vec![("v1", vec![None])], vec![]),
}

fn can_accept_solution_state_impl(routes: Vec<(&str, Vec<Option<&str>>)>, expected: Vec<(&str, &str)>) {
    let total_jobs = get_total_jobs(&routes) + 1;
    let fleet = create_test_fleet();
    let module = GroupModule::new(total_jobs, VIOLATION_CODE, STATE_KEY);
    let mut solution = create_test_solution_context(total_jobs, &fleet, routes);

    module.accept_solution_state(&mut solution);

    compare_actor_groups(&fleet, get_actor_groups(&mut solution, STATE_KEY), expected);
}

parameterized_test! {can_evaluate_job, (routes, route_idx, job_group, expected), {
    can_evaluate_job_impl(routes, route_idx, job_group, expected);
}}

can_evaluate_job! {
    case_01: (vec![("v1", vec![]), ("v2", vec![Some("g1")])], 0, Some("g1"), Some(VIOLATION_CODE)),
    case_02: (vec![("v1", vec![]), ("v2", vec![])], 0, Some("g1"), None),
}

fn can_evaluate_job_impl(
    routes: Vec<(&str, Vec<Option<&str>>)>,
    route_idx: usize,
    job_group: Option<&str>,
    expected: Option<i32>,
) {
    let total_jobs = get_total_jobs(&routes) + 1;
    let fleet = create_test_fleet();
    let solution_ctx = create_test_solution_context(total_jobs, &fleet, routes);
    let route_ctx = solution_ctx.routes.get(route_idx).unwrap();
    let job = Job::Single(create_test_single(job_group));

    let result = GroupHardRouteConstraint { total_jobs, code: VIOLATION_CODE, state_key: STATE_KEY }.evaluate_job(
        &solution_ctx,
        route_ctx,
        &job,
    );

    assert_eq!(result, expected.map(|code| RouteConstraintViolation { code }));
}

parameterized_test! {can_merge_groups, (source, candidate, expected), {
    can_merge_groups_impl(Job::Single(source), Job::Single(candidate), expected);
}}

can_merge_groups! {
    case_01: (create_test_single(Some("group1")), create_test_single(Some("group2")), Err(0)),
    case_02: (create_test_single(Some("group1")), create_test_single(Some("group1")), Ok(())),
    case_03: (create_test_single(None), create_test_single(Some("group1")), Err(0)),
    case_04: (create_test_single(Some("group1")), create_test_single(None), Err(0)),
    case_05: (create_test_single(None), create_test_single(None), Ok(())),
}

fn can_merge_groups_impl(source: Job, candidate: Job, expected: Result<(), i32>) {
    let constraint = GroupModule::new(2, 0, 0);

    let result = constraint.merge(source, candidate).map(|_| ());

    assert_eq!(result, expected);
}
