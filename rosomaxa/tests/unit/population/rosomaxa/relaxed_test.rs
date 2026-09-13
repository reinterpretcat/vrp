use super::*;

fn solution(weight: Float, fitness: Float, infeasibility: Option<Float>) -> RelaxedSolution {
    RelaxedSolution { weights: vec![weight], fitness, infeasibility, progress: None, is_active: false }
}

fn continuation(weight: Float, fitness: Float, infeasibility: Float, episode: usize, step: usize) -> RelaxedSolution {
    RelaxedSolution {
        weights: vec![weight],
        fitness,
        infeasibility: Some(infeasibility),
        progress: Some(RelaxedSolutionProgress::new(episode, step)),
        is_active: true,
    }
}

fn create_relaxed_rosomaxa(
    initial_size: usize,
    selection_size: usize,
) -> Rosomaxa<RelaxedContext, RelaxedTestObjective, RelaxedSolution> {
    let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(selection_size) };
    Rosomaxa::new(RelaxedContext, Arc::new(RelaxedTestObjective), Arc::new(Environment::default()), config).unwrap()
}

fn enter_exploration(
    rosomaxa: &mut Rosomaxa<RelaxedContext, RelaxedTestObjective, RelaxedSolution>,
    initial_size: usize,
) {
    (0..initial_size).for_each(|value| {
        rosomaxa.add(solution(value as Float, value as Float, None));
    });
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
}

#[test]
fn can_keep_relaxed_solutions_hidden_and_bounded() {
    let initial_size = 4;
    let selection_size = 2;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, selection_size);
    enter_exploration(&mut rosomaxa, initial_size);

    for value in 0..20 {
        rosomaxa.add(continuation(value as Float + 0.5, -(value as Float), 0.1 + value as Float, value, 1));
    }

    assert!(rosomaxa.select().all(|solution| solution.infeasibility.is_none()));
    assert!(rosomaxa.ranked().all(|solution| solution.infeasibility.is_none()));
    assert!(rosomaxa.iter().all(|solution| solution.infeasibility.is_none()));

    let reference = rosomaxa.select().next().unwrap();
    assert!(rosomaxa.select_relaxed(reference).is_some());
    let RosomaxaPhases::Exploration { network, .. } = &rosomaxa.phase else { unreachable!() };
    assert!(network.iter_nodes().filter(|node| node.storage.relaxed.len() > 0).count() <= selection_size);
}

#[test]
fn can_preserve_relaxed_archive_during_exploitation() {
    let initial_size = 4;
    let selection_size = 2;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, selection_size);
    enter_exploration(&mut rosomaxa, initial_size);
    rosomaxa.add(continuation(10., 10., 0.1, 1, 1));

    let RosomaxaPhases::Exploration { network, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(network.iter_nodes().filter(|node| node.storage.relaxed.len() > 0).count(), 1);

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..Default::default() });
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
    let reference = rosomaxa.select().next().unwrap();
    assert_eq!(rosomaxa.select_relaxed(reference).unwrap().infeasibility, Some(0.1));

    for value in 0..10 {
        rosomaxa.add(continuation(20. + value as Float, value as Float, 0.2 + value as Float, value + 2, 1));
    }

    let RosomaxaPhases::Exploitation { relaxed, .. } = &rosomaxa.phase else { unreachable!() };
    assert!(relaxed.len() <= selection_size);
    assert!(rosomaxa.iter().all(|solution| solution.infeasibility.is_none()));
}

#[test]
fn can_replay_regular_boundary_and_continuation_with_equal_weights() {
    let initial_size = 4;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, 4);
    enter_exploration(&mut rosomaxa, initial_size);
    rosomaxa.add(solution(0., 1., Some(0.1)));
    rosomaxa.add(continuation(0., -1., 0.2, 1, 1));

    let RosomaxaPhases::Exploration { network, .. } = &mut rosomaxa.phase else { unreachable!() };
    network.smooth(&RelaxedContext, 1, |_| {});

    let node = network.iter_nodes().find(|node| node.storage.relaxed.len() > 0).unwrap();
    assert!(node.storage.regular.size() > 0);
    assert_eq!(node.storage.relaxed.boundary.as_deref().unwrap().fitness, 1.);
    assert_eq!(node.storage.relaxed.continuation.as_deref().unwrap().fitness, -1.);
    assert_eq!(network.iter_nodes().map(|node| node.storage.relaxed.len()).sum::<usize>(), 2);
}

#[test]
fn can_rank_relaxed_boundary_with_original_objective() {
    let objective = Arc::new(AlternativeRelaxedTestObjective { reverse: false });
    let factory = IndividualStorageFactory::<RelaxedContext, _, RelaxedSolution> {
        node_size: 2,
        random: Arc::new(Environment::default()).random.clone(),
        objective,
    };
    let mut storage = factory.eval(&RelaxedContext);

    storage.add(solution(0., 2., Some(0.1)));
    storage.add(solution(0., 1., Some(0.1)));

    assert_eq!(storage.relaxed.boundary.as_deref().unwrap().fitness, 1.);
}

#[test]
fn can_keep_boundary_and_active_continuation() {
    let objective = RelaxedTestObjective;
    let mut relaxed = RelaxedStorage::default();

    relaxed.add(&objective, solution(0., 12., Some(0.05)));
    relaxed.add(&objective, continuation(0., 9., 0.2, 1, 1));

    assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
    assert_eq!(relaxed.continuation.as_deref().unwrap().fitness, 9.);
    assert_eq!(relaxed.select(false).unwrap().fitness, 12.);
    assert_eq!(relaxed.select(true).unwrap().fitness, 9.);
}

#[test]
fn can_skip_expired_boundary_in_favor_of_active_continuation() {
    let objective = RelaxedTestObjective;
    let mut relaxed = RelaxedStorage::default();

    relaxed.add(&objective, continuation(0., 12., 0.05, 1, 1));
    relaxed.boundary.as_deref_mut().unwrap().end_relaxed_continuation();
    relaxed.add(&objective, continuation(0., 9., 0.2, 2, 1));

    assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
    assert_eq!(relaxed.select(false).unwrap().fitness, 9.);
}

#[test]
fn cannot_select_expired_checkpoint_from_archive() {
    let objective = RelaxedTestObjective;
    let reference = solution(0., 0., None);
    let mut expired = continuation(0., 12., 0.05, 1, 1);
    expired.is_active = false;

    assert!(select_relaxed_archive(&[expired], &objective, &reference, false).is_none());
}

#[test]
fn can_advance_continuation_independently_from_boundary_order() {
    let objective = RelaxedTestObjective;
    let mut relaxed = RelaxedStorage::default();

    relaxed.add(&objective, solution(0., 12., Some(0.05)));
    relaxed.add(&objective, continuation(0., 9., 0.2, 1, 1));
    relaxed.add(&objective, continuation(0., 15., 0.3, 1, 2));

    assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
    assert_eq!(relaxed.continuation.as_deref().unwrap().fitness, 15.);
    assert_eq!(relaxed.select(true).unwrap().progress.unwrap().step(), 2);
}

#[test]
fn can_use_boundary_as_continuation_without_duplicate_storage() {
    let objective = RelaxedTestObjective;
    let mut relaxed = RelaxedStorage::default();
    relaxed.add(&objective, continuation(0., 9., 0.1, 1, 1));

    assert_eq!(relaxed.len(), 1);
    assert!(relaxed.continuation.is_none());
    assert_eq!(relaxed.select(true).unwrap().fitness, 9.);
}

#[test]
fn can_replay_historical_boundary_and_active_continuation_in_any_order() {
    let objective = RelaxedTestObjective;
    let mut before = RelaxedStorage::default();
    before.add(&objective, continuation(0., 10., 0.01, 1, 1));
    before.supersede_continuation(RelaxedSolutionProgress::new(1, 2));
    before.add(&objective, continuation(0., 9., 0.02, 1, 2));
    let replay = before.drain_all().collect::<Vec<_>>();

    for order in [[0, 1], [1, 0]] {
        let mut after = RelaxedStorage::default();
        order.into_iter().for_each(|index| after.add(&objective, replay[index].clone()));

        assert_eq!(after.len(), 2);
        assert_eq!(after.select(true).and_then(|solution| solution.progress).map(|progress| progress.step()), Some(2));
    }
}

#[test]
fn can_remove_expired_episode_from_exploration() {
    let initial_size = 4;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, 4);
    enter_exploration(&mut rosomaxa, initial_size);
    let mut expired = continuation(0., 9., 0.1, 1, 1);

    rosomaxa.add(expired.clone());
    expired.is_active = false;
    rosomaxa.add(expired);

    let RosomaxaPhases::Exploration { network, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(network.iter_nodes().map(|node| node.storage.relaxed.len()).sum::<usize>(), 0);
}

#[test]
fn can_remove_expired_episode_from_exploitation() {
    let initial_size = 4;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, 4);
    enter_exploration(&mut rosomaxa, initial_size);
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..Default::default() });
    let mut expired = continuation(0., 9., 0.1, 1, 1);

    rosomaxa.add(expired.clone());
    expired.is_active = false;
    rosomaxa.add(expired);

    let RosomaxaPhases::Exploitation { relaxed, .. } = &rosomaxa.phase else { unreachable!() };
    assert!(relaxed.is_empty());
}

#[test]
fn can_demote_previous_checkpoint_without_losing_boundary() {
    let objective = RelaxedTestObjective;
    let mut relaxed = RelaxedStorage::default();
    relaxed.add(&objective, continuation(0., 9., 0.1, 1, 1));

    relaxed.supersede_continuation(RelaxedSolutionProgress::new(1, 2));

    assert!(relaxed.select(false).is_none());
    assert!(relaxed.select(true).is_none());
    assert!(!relaxed.boundary.as_deref().unwrap().is_active);
}

#[test]
fn can_alternate_boundary_and_continuation_selection() {
    let initial_size = 4;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, 4);
    enter_exploration(&mut rosomaxa, initial_size);
    rosomaxa.add(solution(0., 12., Some(0.05)));
    rosomaxa.add(continuation(0., 9., 0.2, 1, 1));

    let reference = rosomaxa.select().next().unwrap();
    assert_eq!(rosomaxa.select_relaxed(reference).unwrap().fitness, 12.);
    assert_eq!(rosomaxa.select_relaxed(reference).unwrap().fitness, 9.);
}

#[test]
fn can_preserve_relaxed_roles_independently_from_replay_order() {
    let objective = RelaxedTestObjective;
    let candidates = [solution(0., 12., Some(0.1)), continuation(0., 9., 0.2, 1, 2), continuation(0., 13., 0.3, 1, 1)];
    let orders = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];

    for order in orders {
        let mut relaxed = RelaxedStorage::default();
        order.into_iter().for_each(|index| relaxed.add(&objective, candidates[index].clone()));

        assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
        assert_eq!(relaxed.continuation.as_deref().unwrap().fitness, 9.);
    }
}

#[test]
fn can_drain_relaxed_roles_by_rank() {
    let objective = RelaxedTestObjective;
    let mut relaxed = RelaxedStorage::default();
    relaxed.add(&objective, continuation(0., 9., 0.2, 1, 1));

    let drained = relaxed.drain(0..1);
    assert_eq!(drained.iter().map(|solution| solution.fitness).collect::<Vec<_>>(), vec![9.]);
    assert_eq!(relaxed.len(), 0);

    relaxed.add(&objective, solution(0., 12., Some(0.1)));
    relaxed.add(&objective, continuation(0., 9., 0.2, 1, 1));

    let drained = relaxed.drain(1..2);
    assert_eq!(drained.iter().map(|solution| solution.fitness).collect::<Vec<_>>(), vec![9.]);
    assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
    assert!(relaxed.continuation.is_none());
}

#[test]
fn can_transfer_both_relaxed_roles_to_exploitation() {
    let initial_size = 4;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, 4);
    enter_exploration(&mut rosomaxa, initial_size);
    rosomaxa.add(solution(0., 1., Some(0.1)));
    rosomaxa.add(continuation(0., -1., 0.2, 1, 1));

    let RosomaxaPhases::Exploration { network, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(network.iter_nodes().map(|node| node.storage.relaxed.len()).sum::<usize>(), 2);

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..Default::default() });

    let RosomaxaPhases::Exploitation { relaxed, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(relaxed.len(), 2);
    let reference = rosomaxa.select().next().unwrap();
    assert!(rosomaxa.select_relaxed(reference).is_some());
}

#[test]
fn can_bound_relaxed_archive_when_exploration_ends() {
    let initial_size = 4;
    let selection_size = 2;
    let mut rosomaxa = create_relaxed_rosomaxa(initial_size, selection_size);
    enter_exploration(&mut rosomaxa, initial_size);

    let RosomaxaPhases::Exploration { network, .. } = &mut rosomaxa.phase else { unreachable!() };
    network.iter_nodes_mut().enumerate().for_each(|(index, node)| {
        let value = index as Float;
        node.storage.relaxed.boundary = Some(Box::new(solution(value, value, Some(0.1))));
        node.storage.relaxed.continuation = Some(Box::new(continuation(value + 0.1, -value - 1., 0.2, index, 1)));
    });
    assert!(network.iter_nodes().map(|node| node.storage.relaxed.len()).sum::<usize>() > initial_size);

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..Default::default() });

    let RosomaxaPhases::Exploitation { relaxed, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(relaxed.len(), selection_size);
}

#[test]
fn can_keep_actionable_continuations_when_pruning_flat_archive() {
    let objective = RelaxedTestObjective;
    let mut archive = vec![
        continuation(0., 1., 0., 1, 1),
        continuation(100., 1., 0., 2, 1),
        continuation(0.01, 1., 0.01, 1, 2),
        continuation(99.99, 1., 0.01, 2, 2),
    ];
    archive[0].is_active = false;
    archive[1].is_active = false;

    prune_relaxed_archive(&mut archive, &objective, 2);

    assert_eq!(archive.len(), 2);
    assert!(archive.iter().all(RelaxedSolution::is_relaxed_continuation));
}
