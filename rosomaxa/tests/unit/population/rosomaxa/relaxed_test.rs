use super::*;

#[test]
fn can_keep_relaxed_solutions_hidden_and_bounded() {
    let initial_size = 4;
    let selection_size = 2;
    let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(selection_size) };
    let mut rosomaxa =
        Rosomaxa::new(RelaxedContext, Arc::new(RelaxedTestObjective), Arc::new(Environment::default()), config)
            .unwrap();

    for value in 0..initial_size {
        rosomaxa.add(RelaxedSolution { weights: vec![value as Float], fitness: value as Float, infeasibility: None });
    }
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

    for value in 0..20 {
        rosomaxa.add(RelaxedSolution {
            weights: vec![value as Float + 0.5],
            fitness: -(value as Float),
            infeasibility: Some(0.1 + value as Float),
        });
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
    let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(selection_size) };
    let mut rosomaxa =
        Rosomaxa::new(RelaxedContext, Arc::new(RelaxedTestObjective), Arc::new(Environment::default()), config)
            .unwrap();

    for value in 0..initial_size {
        rosomaxa.add(RelaxedSolution { weights: vec![value as Float], fitness: value as Float, infeasibility: None });
    }
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
    rosomaxa.add(RelaxedSolution { weights: vec![10.], fitness: 10., infeasibility: Some(0.1) });
    let RosomaxaPhases::Exploration { network, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(network.iter_nodes().filter(|node| node.storage.relaxed.len() > 0).count(), 1);

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..Default::default() });
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
    let reference = rosomaxa.select().next().unwrap();
    assert_eq!(rosomaxa.select_relaxed(reference).unwrap().infeasibility, Some(0.1));

    for value in 0..10 {
        rosomaxa.add(RelaxedSolution {
            weights: vec![20. + value as Float],
            fitness: value as Float,
            infeasibility: Some(0.2 + value as Float),
        });
    }

    let RosomaxaPhases::Exploitation { relaxed, .. } = &rosomaxa.phase else { unreachable!() };
    assert!(relaxed.len() <= selection_size);
    assert!(rosomaxa.iter().all(|solution| solution.infeasibility.is_none()));
}

#[test]
fn can_replay_regular_and_relaxed_solutions_with_equal_weights() {
    let initial_size = 4;
    let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(4) };
    let mut rosomaxa =
        Rosomaxa::new(RelaxedContext, Arc::new(RelaxedTestObjective), Arc::new(Environment::default()), config)
            .unwrap();

    for value in 0..initial_size {
        rosomaxa.add(RelaxedSolution { weights: vec![value as Float], fitness: value as Float, infeasibility: None });
    }
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
    rosomaxa.add(RelaxedSolution { weights: vec![0.], fitness: 1., infeasibility: Some(0.1) });
    rosomaxa.add(RelaxedSolution { weights: vec![0.], fitness: -1., infeasibility: Some(0.2) });

    let RosomaxaPhases::Exploration { network, .. } = &mut rosomaxa.phase else { unreachable!() };
    network.smooth(&RelaxedContext, 1, |_| {});

    let node = network.iter_nodes().find(|node| node.storage.relaxed.len() > 0).unwrap();
    assert!(node.storage.regular.size() > 0);
    assert_eq!(node.storage.relaxed.boundary.as_deref().unwrap().weights, vec![0.]);
    assert_eq!(node.storage.relaxed.bridge.as_deref().unwrap().weights, vec![0.]);
    assert_eq!(network.iter_nodes().map(|node| node.storage.relaxed.len()).sum::<usize>(), 2);
}

#[test]
fn can_rank_relaxed_resident_with_original_objective() {
    let objective = Arc::new(AlternativeRelaxedTestObjective { reverse: false });
    let factory = IndividualStorageFactory::<RelaxedContext, _, RelaxedSolution> {
        node_size: 2,
        random: Arc::new(Environment::default()).random.clone(),
        objective,
    };
    let mut storage = factory.eval(&RelaxedContext);

    storage.add(RelaxedSolution { weights: vec![0.], fitness: 2., infeasibility: Some(0.1) });
    storage.add(RelaxedSolution { weights: vec![0.], fitness: 1., infeasibility: Some(0.1) });

    assert_eq!(storage.relaxed.boundary.as_deref().unwrap().fitness, 1.);
}

#[test]
fn can_classify_bridge_against_best_regular_solution_in_original_objective() {
    let objective = Arc::new(AlternativeRelaxedTestObjective { reverse: false });
    let factory = IndividualStorageFactory::<RelaxedContext, _, RelaxedSolution> {
        node_size: 2,
        random: Arc::new(Environment::default()).random.clone(),
        objective,
    };
    let mut storage = factory.eval(&RelaxedContext);

    // The node's alternative objective prefers fitness 3, but fitness 1 is the feasible reference under the
    // original objective. An infeasible solution at 2 must therefore remain a boundary, not a bridge.
    storage.add(RelaxedSolution { weights: vec![1.], fitness: 1., infeasibility: None });
    storage.add(RelaxedSolution { weights: vec![3.], fitness: 3., infeasibility: None });
    storage.add(RelaxedSolution { weights: vec![2.], fitness: 2., infeasibility: Some(0.1) });

    assert_eq!(storage.regular.best().unwrap().fitness, 3.);
    assert_eq!(storage.regular.best_with_objective(storage.relaxed_objective.as_ref()).unwrap().fitness, 1.);
    assert_eq!(storage.relaxed.boundary.as_deref().unwrap().fitness, 2.);
    assert!(storage.relaxed.bridge.is_none());
}

#[test]
fn can_keep_relaxed_boundary_and_objective_bridge() {
    let objective = Arc::new(RelaxedTestObjective);
    let factory = IndividualStorageFactory::<RelaxedContext, _, RelaxedSolution> {
        node_size: 2,
        random: Arc::new(Environment::default()).random.clone(),
        objective,
    };
    let mut storage = factory.eval(&RelaxedContext);

    storage.add(RelaxedSolution { weights: vec![0.], fitness: 10., infeasibility: None });
    storage.add(RelaxedSolution { weights: vec![0.], fitness: 12., infeasibility: Some(0.1) });
    storage.add(RelaxedSolution { weights: vec![0.], fitness: 9., infeasibility: Some(0.05) });

    let reference = RelaxedSolution { weights: vec![0.], fitness: 10., infeasibility: None };
    assert_eq!(storage.relaxed.boundary.as_deref().unwrap().fitness, 12.);
    assert_eq!(storage.relaxed.bridge.as_deref().unwrap().fitness, 9.);
    assert_eq!(storage.relaxed.select(storage.relaxed_objective.as_ref(), &reference).unwrap().fitness, 9.);

    // Once the feasible incumbent overtakes the bridge, it competes for the boundary slot by violation.
    storage.add(RelaxedSolution { weights: vec![1.], fitness: 8., infeasibility: None });
    assert!(storage.relaxed.bridge.is_none());
    assert_eq!(storage.relaxed.boundary.as_deref().unwrap().fitness, 9.);
}

#[test]
fn can_preserve_relaxed_tradeoff_until_regular_solution_arrives() {
    let objective = RelaxedTestObjective;
    let regular = RelaxedSolution { weights: vec![0.], fitness: 10., infeasibility: None };
    let candidates = [
        RelaxedSolution { weights: vec![0.], fitness: 12., infeasibility: Some(0.1) },
        RelaxedSolution { weights: vec![0.], fitness: 9., infeasibility: Some(0.2) },
        RelaxedSolution { weights: vec![0.], fitness: 13., infeasibility: Some(0.3) },
    ];
    let orders = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];

    for order in orders {
        for regular_position in 0..=candidates.len() {
            let mut relaxed = RelaxedStorage::default();
            let mut has_regular = false;
            let mut candidate_position = 0;

            for position in 0..=candidates.len() {
                if position == regular_position {
                    has_regular = true;
                    relaxed.on_regular_updated(&objective, Some(&regular));
                } else {
                    let candidate = candidates[order[candidate_position]].clone();
                    relaxed.add(&objective, has_regular.then_some(&regular), candidate);
                    candidate_position += 1;
                }
            }

            assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
            assert_eq!(relaxed.bridge.as_deref().unwrap().fitness, 9.);
        }
    }
}

#[test]
fn can_drain_relaxed_roles_by_rank() {
    let objective = RelaxedTestObjective;
    let regular = RelaxedSolution { weights: vec![0.], fitness: 10., infeasibility: None };
    let mut relaxed = RelaxedStorage::default();

    relaxed.add(
        &objective,
        Some(&regular),
        RelaxedSolution { weights: vec![0.], fitness: 9., infeasibility: Some(0.2) },
    );

    let drained = relaxed.drain(0..1);
    assert_eq!(drained.iter().map(|solution| solution.fitness).collect::<Vec<_>>(), vec![9.]);
    assert_eq!(relaxed.len(), 0);

    relaxed.add(
        &objective,
        Some(&regular),
        RelaxedSolution { weights: vec![0.], fitness: 12., infeasibility: Some(0.1) },
    );
    relaxed.add(
        &objective,
        Some(&regular),
        RelaxedSolution { weights: vec![0.], fitness: 9., infeasibility: Some(0.2) },
    );

    let drained = relaxed.drain(1..2);
    assert_eq!(drained.iter().map(|solution| solution.fitness).collect::<Vec<_>>(), vec![9.]);
    assert_eq!(relaxed.boundary.as_deref().unwrap().fitness, 12.);
    assert!(relaxed.bridge.is_none());
}

#[test]
fn can_transfer_both_relaxed_roles_to_exploitation() {
    let initial_size = 4;
    let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(4) };
    let mut rosomaxa =
        Rosomaxa::new(RelaxedContext, Arc::new(RelaxedTestObjective), Arc::new(Environment::default()), config)
            .unwrap();

    for value in 0..initial_size {
        rosomaxa.add(RelaxedSolution { weights: vec![value as Float], fitness: value as Float, infeasibility: None });
    }
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

    rosomaxa.add(RelaxedSolution { weights: vec![0.], fitness: 1., infeasibility: Some(0.1) });
    rosomaxa.add(RelaxedSolution { weights: vec![0.], fitness: -1., infeasibility: Some(0.2) });

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
    let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(selection_size) };
    let mut rosomaxa =
        Rosomaxa::new(RelaxedContext, Arc::new(RelaxedTestObjective), Arc::new(Environment::default()), config)
            .unwrap();

    for value in 0..initial_size {
        rosomaxa.add(RelaxedSolution { weights: vec![value as Float], fitness: value as Float, infeasibility: None });
    }
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

    let RosomaxaPhases::Exploration { network, .. } = &mut rosomaxa.phase else { unreachable!() };
    network.iter_nodes_mut().enumerate().for_each(|(index, node)| {
        let value = index as Float;
        node.storage.relaxed.boundary =
            Some(Box::new(RelaxedSolution { weights: vec![value], fitness: value, infeasibility: Some(0.1) }));
        node.storage.relaxed.bridge = Some(Box::new(RelaxedSolution {
            weights: vec![value + 0.1],
            fitness: -value - 1.,
            infeasibility: Some(0.2),
        }));
    });
    assert!(network.iter_nodes().map(|node| node.storage.relaxed.len()).sum::<usize>() > initial_size);

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..Default::default() });

    let RosomaxaPhases::Exploitation { relaxed, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(relaxed.len(), selection_size);
}
