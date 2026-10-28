use super::*;
use crate::helpers::solver::generate_matrix_routes_with_defaults;
use rosomaxa::prelude::Environment;

fn create_parent_pair() -> (InsertionContext, InsertionContext, Job, Job) {
    let (problem, solution) = generate_matrix_routes_with_defaults(3, 2, false);
    let problem = Arc::new(problem);
    let environment = Arc::new(Environment::default());
    let source = InsertionContext::new_from_solution(problem, (solution, None), environment);
    let mut target = source.deep_copy();

    let displaced = target.solution.routes[0].route().tour.jobs().next().unwrap().clone();
    let transferred = target.solution.routes[1].route().tour.jobs().next().unwrap().clone();
    let activity = target.solution.routes[1].route().tour.job_activities(&transferred).next().unwrap().clone();

    assert!(target.solution.routes[0].route_mut().tour.remove(&displaced));
    assert!(target.solution.routes[1].route_mut().tour.remove(&transferred));
    target.solution.routes[0].route_mut().tour.insert_last(activity);
    target.problem.goal.accept_solution_state(&mut target.solution);

    (source, target, displaced, transferred)
}

#[test]
fn can_transplant_target_route_without_rebuilding_solution() {
    let (source, target, _, _) = create_parent_pair();
    let route_pair = select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new()).remove(0);
    let target_jobs = route_pair.target_route.route().tour.jobs().cloned().collect::<HashSet<_>>();
    let displaced = source.solution.routes[route_pair.source_idx]
        .route()
        .tour
        .jobs()
        .filter(|job| !target_jobs.contains(*job))
        .cloned()
        .collect::<Vec<_>>();

    let candidate = transplant_route(&source, route_pair.source_idx, route_pair.target_route).unwrap();
    let transplanted = &candidate.solution.routes[route_pair.source_idx];

    assert!(target_jobs.iter().all(|job| transplanted.route().tour.contains(job)));
    assert!(
        target_jobs.iter().all(|job| {
            candidate.solution.routes.iter().filter(|route| route.route().tour.contains(job)).count() == 1
        })
    );
    assert!(displaced.iter().all(|job| candidate.solution.required.contains(job)));
}

#[test]
fn can_measure_structural_difference_between_parents() {
    let (source, target, _, _) = create_parent_pair();
    let source_structure = SolutionStructure::new(&source);

    assert_eq!(source_structure.distance(&source_structure).attributes(), 0);

    let distance = source_structure.distance(&SolutionStructure::new(&target));
    assert!(distance.route_partition > 0);
    assert!(distance.adjacency > 0);
    assert!(distance.score() > 0.);
}

#[test]
fn skips_routes_with_locked_jobs() {
    let (mut source, target, _, transferred) = create_parent_pair();
    source.solution.locked.insert(transferred.clone());

    let route_pairs = select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new());

    assert!(!route_pairs.is_empty());
    for route_pair in route_pairs {
        assert!(!source.solution.routes[route_pair.source_idx].route().tour.contains(&transferred));
        assert!(!route_pair.target_route.route().tour.contains(&transferred));
    }
}

#[test]
fn respects_locks_from_both_endpoints() {
    let (source, mut target, _, transferred) = create_parent_pair();
    target.solution.locked.insert(transferred.clone());

    let route_pairs = select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new());

    assert!(!route_pairs.is_empty());
    for route_pair in route_pairs {
        assert!(!source.solution.routes[route_pair.source_idx].route().tour.contains(&transferred));
        assert!(!route_pair.target_route.route().tour.contains(&transferred));
    }
}

#[test]
fn respects_affected_activity_limit() {
    let (source, target, _, _) = create_parent_pair();

    assert!(select_route_pairs(&source, &target, 1, &HashSet::new(), &HashSet::new()).is_empty());
    assert!(!select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new()).is_empty());
}

#[test]
fn does_not_use_empty_guide_route() {
    let (source, mut target, _, _) = create_parent_pair();
    let removed = target.solution.routes[0].route().tour.jobs().cloned().collect::<Vec<_>>();
    removed.iter().for_each(|job| assert!(target.solution.routes[0].route_mut().tour.remove(job)));
    target.problem.goal.accept_route_state(&mut target.solution.routes[0]);
    target.problem.goal.accept_solution_state(&mut target.solution);

    let route_pairs = select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new());

    assert!(!route_pairs.is_empty());
    assert!(route_pairs.iter().all(|route_pair| route_pair.target_idx != 0));
}

#[test]
fn does_not_reuse_committed_routes() {
    let (source, target, _, _) = create_parent_pair();
    let first = select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new()).remove(0);
    let committed_source_routes = HashSet::from([first.source_idx]);
    let committed_target_routes = HashSet::from([first.target_idx]);

    let remaining =
        select_route_pairs(&source, &target, usize::MAX, &committed_source_routes, &committed_target_routes);

    assert!(!remaining.is_empty());
    assert!(remaining.iter().all(|pair| pair.source_idx != first.source_idx));
    assert!(remaining.iter().all(|pair| pair.target_idx != first.target_idx));
}

#[test]
fn does_not_overwrite_committed_route_block() {
    let (source, target, _, _) = create_parent_pair();
    let first = select_route_pairs(&source, &target, usize::MAX, &HashSet::new(), &HashSet::new()).remove(0);
    let partial = transplant_route(&source, first.source_idx, first.target_route).unwrap();
    let committed_jobs = get_ordered_jobs(&partial.solution.routes[first.source_idx]);
    let committed_source_routes = HashSet::from([first.source_idx]);
    let committed_target_routes = HashSet::from([first.target_idx]);
    let next = select_route_pairs(&partial, &target, usize::MAX, &committed_source_routes, &committed_target_routes)
        .into_iter()
        .find_map(|route_pair| transplant_route(&partial, route_pair.source_idx, route_pair.target_route))
        .unwrap();

    assert_eq!(get_ordered_jobs(&next.solution.routes[first.source_idx]), committed_jobs);
}

#[test]
fn selects_quality_half_before_structural_filtering() {
    let values = vec![6, 1, 5, 2, 4, 3];

    let selected = select_quality_half(values, Ord::cmp);

    assert_eq!(selected, vec![1, 2, 3]);
}
